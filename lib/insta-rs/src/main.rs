use anyhow::{
    bail,
    ensure,
    Context,
};
use directories_next::ProjectDirs;
use insta::{
    Client,
    CookieStore,
    MediaType,
};
use std::path::Path;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
};
use url::Url;

#[derive(Debug, argh::FromArgs)]
#[argh(description = "a tool to interface with instagram")]
struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Login(LoginOptions),
    Download(DownloadOptions),
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "login", description = "log in to instagram")]
struct LoginOptions {
    #[argh(option, description = "the username")]
    username: String,

    #[argh(option, description = "the password")]
    password: String,
}

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "download",
    description = "download a post from instagram"
)]
struct DownloadOptions {
    #[argh(positional, description = "the post url")]
    post: String,
}

struct BoxError(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Debug for BoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for BoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BoxError {}

/// Config
pub struct Config {
    document: toml_edit::Document,
}

impl Config {
    /// Load config
    async fn load(path: &Path) -> anyhow::Result<Self> {
        let data = match tokio::fs::read_to_string(path).await {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return Err(e).context("failed to read config")?;
            }
        };

        Ok(Self {
            document: data.parse().context("failed to parse toml")?,
        })
    }

    /// Save config to a file
    async fn save(&self, path: &Path) -> anyhow::Result<()> {
        let data = self.document.to_string();
        tokio::fs::write(path, data)
            .await
            .context("failed to write to file")?;
        Ok(())
    }

    /// Get the username
    fn get_username(&self) -> Option<&str> {
        self.document.get("username")?.as_str()
    }

    /// Get the password
    fn get_password(&self) -> Option<&str> {
        self.document.get("password")?.as_str()
    }

    /// Set the username
    fn set_username(&mut self, username: &str) {
        self.document.insert("username", toml_edit::value(username));
    }

    /// Set the password
    fn set_password(&mut self, password: &str) {
        self.document.insert("password", toml_edit::value(password));
    }
}

fn main() {
    let options = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            1
        }
    };

    std::process::exit(code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    tokio_rt.block_on(async_main(options))?;

    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let project_dirs = ProjectDirs::from("", "", "insta").context("missing project dirs")?;
    let config_dir = project_dirs.config_dir();
    let cache_dir = project_dirs.cache_dir();

    tokio::fs::create_dir_all(&config_dir)
        .await
        .context("failed to create config dir")?;
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .context("failed to create cache dir")?;

    let config_file_name = "config.toml";
    let config_path = config_dir.join(config_file_name);

    let session_file_name = "session.json";
    let session_file_path = cache_dir.join(session_file_name);

    let client = Client::new();
    let mut config = Config::load(&config_path)
        .await
        .context("failed to load config")?;

    let maybe_username = config.get_username();
    let maybe_password = config.get_password();

    let maybe_cookie_store = {
        let session_file_path = session_file_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            use std::{
                fs::File,
                io::BufReader,
            };

            match File::open(&session_file_path).map(BufReader::new) {
                Ok(mut file) => Ok(Some(
                    CookieStore::load_json(&mut file)
                        .map_err(BoxError)
                        .context("failed to load session")?,
                )),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e).context("failed to open session file"),
            }
        })
        .await?
        .context("failed to load session file")?
    };

    match maybe_cookie_store {
        Some(mut cookie_store) => std::mem::swap(
            &mut *client.cookie_store.lock().expect("cookie store poisoned"),
            &mut cookie_store,
        ),
        None => {
            if let (Some(username), Some(password)) = (maybe_username, maybe_password) {
                println!("Missing session file, logging in...");
                let login_response = client
                    .login(username, password)
                    .await
                    .context("failed to login")?;
                ensure!(
                    login_response.authenticated,
                    "login failed to authenticate user"
                );

                let client = client.clone();
                tokio::task::spawn_blocking(move || {
                    use std::fs::File;

                    let mut file = File::create(&session_file_path)
                        .context("failed to create session file")?;
                    let cookie_store = client.cookie_store.lock().expect("cookie store poisoned");
                    cookie_store
                        .save_json(&mut file)
                        .map_err(BoxError)
                        .context("failed to save session file")?;

                    Result::<_, anyhow::Error>::Ok(())
                })
                .await??;
            } else {
                println!("Skipping log-in as username and password are not specified");
            }
        }
    };

    match options.subcommand {
        Subcommand::Login(options) => {
            config.set_username(&options.username);
            config.set_password(&options.password);

            config
                .save(&config_path)
                .await
                .context("failed to save config")?;

            // TODO: Login, only save if login was valid
        }
        Subcommand::Download(options) => {
            let post_page = client
                .get_post(&options.post)
                .await
                .context("failed to get post")?;

            let post_page_item = post_page.items.first().context("missing post item")?;

            match post_page_item.media_type {
                MediaType::Photo => {
                    let image_versions2_candidate = post_page_item
                        .get_best_image_versions2_candidate()
                        .context("failed to select an image_versions2_candidate")?;

                    let extension = get_extension_from_url(&image_versions2_candidate.url)
                        .context("missing image extension")?;
                    let file_name = format!("{}.{}", post_page_item.code, extension);
                    let mut file = tokio::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(file_name)
                        .await
                        .context("failed to open output file")?;

                    download_to_file(
                        &client.client,
                        image_versions2_candidate.url.as_str(),
                        &mut file,
                    )
                    .await
                    .context("failed to download")?;
                }

                MediaType::Video => {
                    let video_version = post_page_item
                        .get_best_video_version()
                        .context("failed to get the best video version")?;

                    let extension =
                        get_extension_from_url(&video_version.url).context("missing extension")?;
                    let file_name = format!("{}.{}", post_page_item.code, extension);
                    let mut file = tokio::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(file_name)
                        .await
                        .context("failed to open output file")?;

                    download_to_file(&client.client, video_version.url.as_str(), &mut file)
                        .await
                        .context("failed to download")?;
                }
                MediaType::Carousel => {
                    for (i, item) in post_page_item
                        .carousel_media
                        .as_ref()
                        .context("missing carousel media")?
                        .iter()
                        .enumerate()
                    {
                        match item.media_type {
                            MediaType::Photo => {
                                let image_versions2_candidate = item
                                    .get_best_image_versions2_candidate()
                                    .context("failed to select an image_versions2_candidate")?;

                                let extension =
                                    get_extension_from_url(&image_versions2_candidate.url)
                                        .context("missing image extension")?;
                                let file_name =
                                    format!("{}.{}.{}", post_page_item.code, i + 1, extension);
                                let mut file = tokio::fs::OpenOptions::new()
                                    .create_new(true)
                                    .write(true)
                                    .open(file_name)
                                    .await
                                    .context("failed to open output file")?;

                                download_to_file(
                                    &client.client,
                                    image_versions2_candidate.url.as_str(),
                                    &mut file,
                                )
                                .await
                                .context("failed to download")?;
                            }
                            _ => {
                                bail!("Unsupported media_type `{:?}`", item.media_type);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_extension_from_url(url: &Url) -> Option<&str> {
    Some(url.path_segments()?.rev().next()?.rsplit_once('.')?.1)
}

/// Try to download a url to a file. This will NOT delete the file if it fails.
pub(crate) async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    file: &mut File,
) -> anyhow::Result<()> {
    // Start request
    let mut response = client.get(url).send().await?.error_for_status()?;

    // If there is a content-length, stash it and pre-allocate space in the file.
    let maybe_content_length = response.content_length();
    if let Some(content_length) = maybe_content_length {
        file.set_len(content_length).await?;
    }

    // Perform the download, keeping track of the number of bytes written.
    let mut actual_length = 0;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        actual_length += u64::try_from(chunk.len())?;
    }

    // If a pre-allocation occured and if the actual length differs from the reported content length,
    // return an error.
    if let Some(content_length) = maybe_content_length {
        ensure!(
            actual_length == content_length,
            "reported content length ({}) is different from the actual length ({})",
            content_length,
            actual_length
        );
    }

    // flush and sync the file contents and metadata to the disk
    file.flush().await?;
    file.sync_all().await?;

    Ok(())
}
