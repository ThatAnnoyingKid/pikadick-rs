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
    Saved(SavedOptions),
    GetMediaInfo(GetMediaInfoOptions),
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

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "saved", description = "interact with saved posts")]
struct SavedOptions {
    #[argh(subcommand)]
    subcommand: SavedOptionsSubcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum SavedOptionsSubcommand {
    Unsave(UnsaveOptions),
    Get(GetOptions),
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand, name = "unsave", description = "Unsave a saved post")]
struct UnsaveOptions {
    #[argh(positional, description = "the media id of the post to unsave")]
    media_id: u64,
}

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "get",
    description = "Get saved posts for the current user"
)]
struct GetOptions {
    #[argh(
        option,
        short = 'n',
        long = "num-posts",
        description = "the number of posts to retrieve",
        default = "12"
    )]
    num_posts: u32,

    #[argh(option, short = 'a', long = "after", description = "the after marker")]
    after: Option<String>,
}

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "get-media-info",
    description = "Get the media info for the post with the given media id"
)]
struct GetMediaInfoOptions {
    #[argh(positional, description = "the media id")]
    media_id: u64,
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

fn main() -> anyhow::Result<()> {
    // Run this first, as this will exit the process without running destructors on failure.
    let options = argh::from_env();
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
                Ok(mut file) => {
                    let cookie_store = CookieStore::load_json(&mut file)
                        .map_err(BoxError)
                        .context("failed to load session")?;

                    Ok(Some(cookie_store))
                }
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
                println!("missing session file, logging in...");
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
                println!("skipping log-in as username and password are not specified");
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
                .get_post_page(&options.post)
                .await
                .context("failed to get post page")?;
            let media_info = client
                .get_media_info(post_page.media_id)
                .await
                .context("failed to get media info")?;

            let media_item = media_info.items.first().context("missing post item")?;
            ensure!(media_info.items.len() == 1);

            let mut download_items = Vec::with_capacity(4);

            match media_item.media_type {
                MediaType::Photo => {
                    let image_versions2_candidate = media_item
                        .get_best_image_versions2_candidate()
                        .context("failed to select an image_versions2_candidate")?;
                    let url = &image_versions2_candidate.url;
                    let extension =
                        get_extension_from_url(url).context("missing image extension")?;
                    let file_name = format!("{}.{}", media_item.code, extension);

                    download_items.push((url, file_name));
                }
                MediaType::Video => {
                    let video_version = media_item
                        .get_best_video_version()
                        .context("failed to get the best video version")?;
                    let url = &video_version.url;

                    let extension = get_extension_from_url(url).context("missing extension")?;
                    let file_name = format!("{}.{}", media_item.code, extension);

                    download_items.push((url, file_name));
                }
                MediaType::Carousel => {
                    for (i, item) in media_item
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
                                let url = &image_versions2_candidate.url;

                                let extension = get_extension_from_url(url)
                                    .context("missing image extension")?;
                                let file_name =
                                    format!("{}.{}.{}", media_item.code, i + 1, extension);

                                download_items.push((url, file_name));
                            }
                            MediaType::Video => {
                                let video_version = item
                                    .get_best_video_version()
                                    .context("failed to get the best video version")?;
                                let url = &video_version.url;

                                let extension =
                                    get_extension_from_url(url).context("missing extension")?;
                                let file_name = format!("{}.{}", media_item.code, extension);

                                download_items.push((url, file_name));
                            }
                            _ => {
                                bail!("Unsupported media_type `{:?}`", item.media_type);
                            }
                        }
                    }
                }
            }

            for (url, file_name) in download_items {
                println!("downloading `{file_name}`...");
                let downloaded = download_to_path(&client.client, url.as_str(), file_name.as_ref())
                    .await
                    .context("failed to download")?;

                if !downloaded {
                    println!("  skipped downloading as it already exists...");
                }
            }
        }
        Subcommand::Saved(options) => match options.subcommand {
            SavedOptionsSubcommand::Unsave(options) => {
                client.unsave_post(options.media_id).await?;
                println!("unsaved post `{}`", options.media_id);
            }
            SavedOptionsSubcommand::Get(options) => {
                let saved_posts = client
                    .get_saved_posts(options.num_posts, options.after.as_deref())
                    .await
                    .context("failed to get saved posts")?;

                let edge_saved_media = &saved_posts.data.user.edge_saved_media;
                let num_posts = edge_saved_media.count;

                println!("total # of saved posts: {num_posts}");
                println!("# of posts retrieved: {}", edge_saved_media.edges.len());
                println!("end cursor: {}", edge_saved_media.page_info.end_cursor);
                println!(
                    "has next page: {}",
                    edge_saved_media.page_info.has_next_page
                );
                println!();

                for node in edge_saved_media.edges.iter().map(|edge| &edge.node) {
                    println!("id: {}", node.id);
                    println!("shortcode: {}", node.shortcode);
                    println!("is video: {}", node.is_video);
                    println!("owner id: {}", node.owner.id);
                    if let Some(accessibility_caption) = node.accessibility_caption.as_deref() {
                        println!("accessibility caption: {accessibility_caption}");
                    }
                    {
                        let edges = &node.edge_media_to_caption.edges;
                        ensure!(edges.len() <= 1);

                        if let Some(caption) = edges.get(0).map(|edge| &edge.node.text) {
                            println!("edge media to caption: {caption}");
                        }
                    }
                    println!();
                }
            }
        },
        Subcommand::GetMediaInfo(options) => {
            let media_info = client
                .get_media_info(options.media_id)
                .await
                .context("failed to get media info")?;
            let media_item = media_info.items.first().context("missing post item")?;
            ensure!(media_info.items.len() == 1);

            println!("username: {}", media_item.user.username);
            println!("user id: {}", media_item.user.pk);
            println!("user full name: {}", media_item.user.full_name);
        }
    }

    Ok(())
}

async fn download_to_path(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
) -> anyhow::Result<bool> {
    match tokio::fs::metadata(path).await {
        Ok(_metadata) => {
            return Ok(false);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Pass
        }
        Err(e) => {
            return Err(e).context("failed to stat");
        }
    }

    let tmp_path = pikadick_util::with_push_extension(path, "part");
    let mut tmp_file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp_path)
        .await
        .context("failed to open tmp file")?;
    let mut tmp_path = pikadick_util::DropRemovePath::new(tmp_path);
    pikadick_util::download_to_file(client, url, &mut tmp_file)
        .await
        .context("failed to download to path")?;
    tokio::fs::rename(&tmp_path, &path)
        .await
        .context("failed to rename file")?;
    tmp_path.persist();

    Ok(true)
}

fn get_extension_from_url(url: &Url) -> Option<&str> {
    Some(url.path_segments()?.rev().next()?.rsplit_once('.')?.1)
}
