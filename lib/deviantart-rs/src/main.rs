use anyhow::Context;
use std::{
    fmt::Write,
    path::{
        Path,
        PathBuf,
    },
};

#[derive(argh::FromArgs)]
#[argh(description = "a tool to interact with deviantart")]
struct Options {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    Login(LoginOptions),
    Search(SearchOptions),
    Download(DownloadOptions),
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "login")]
#[argh(description = "login on deviantart")]
struct LoginOptions {
    #[argh(option, description = "your username", short = 'u', long = "username")]
    username: String,

    #[argh(option, description = "your password", short = 'p', long = "password")]
    password: String,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "search")]
#[argh(description = "search on deviantart")]
struct SearchOptions {
    #[argh(positional, description = "the query string")]
    query: String,

    #[argh(switch, long = "no-login", description = "do not try to log in")]
    no_login: bool,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download")]
#[argh(description = "download from deviantart")]
struct DownloadOptions {
    #[argh(positional, description = "the deviation url")]
    url: String,

    #[argh(
        switch,
        description = "allow using  the fullview deviantart url, which is lower quality"
    )]
    allow_fullview: bool,

    #[argh(switch, long = "no-login", description = "do not try to log in")]
    no_login: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Config {
    fn new() -> Self {
        Config {
            username: None,
            password: None,
        }
    }

    async fn get_config_path() -> anyhow::Result<PathBuf> {
        let base_dirs = directories_next::BaseDirs::new().context("failed to get base dirs")?;
        let dir_path = base_dirs.config_dir().join("deviantart");
        tokio::fs::create_dir_all(&dir_path)
            .await
            .context("failed to create config dir")?;
        let config_path = dir_path.join("config.toml");
        Ok(config_path)
    }

    async fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path().await?;
        let mut new_config = Self::load().await.unwrap_or_else(|_| Self::new());

        if let Some(username) = self.username.clone() {
            new_config.username = Some(username);
        }

        if let Some(password) = self.password.clone() {
            new_config.password = Some(password);
        }

        let toml_str = toml::to_string_pretty(&new_config).context("failed to serialize config")?;

        tokio::fs::write(config_path, toml_str)
            .await
            .context("failed to write config")?;

        Ok(())
    }

    async fn load() -> anyhow::Result<Self> {
        let config_path = Self::get_config_path().await?;

        let config_str = tokio::fs::read_to_string(config_path)
            .await
            .context("failed to read config file")?;
        toml::from_str(&config_str).context("failed to parse config")
    }
}

fn main() {
    let exit_code = {
        let options: Options = argh::from_env();

        if let Err(e) = real_main(options) {
            eprintln!("{:?}", e);
            1
        } else {
            0
        }
    };
    std::process::exit(exit_code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start tokio runtime")?;

    tokio_rt.block_on(async_main(options))?;
    eprintln!("Done.");

    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = deviantart::Client::new();

    match options.subcommand {
        SubCommand::Login(options) => {
            let mut config = Config::new();
            config.username = Some(options.username);
            config.password = Some(options.password);
            config.save().await.context("failed to save config")?;
            if let Err(e) = tokio::fs::remove_file(get_cookie_file_path()?).await {
                eprintln!("Failed to delete old cookie file: {}", e);
            }
        }
        SubCommand::Search(options) => {
            let config = Config::load()
                .await
                .map(|config| {
                    println!("loaded config");
                    config
                })
                .unwrap_or_else(|e| {
                    println!("failed to load config: {:?}", e);
                    Config::new()
                });
            println!();

            if !options.no_login {
                try_signin_cli(
                    &client,
                    config.username.as_deref(),
                    config.password.as_deref(),
                )
                .await?;
            }

            let results = client
                .search(&options.query)
                .await
                .with_context(|| format!("failed to search for '{}'", &options.query))?;

            if results.deviations.is_empty() {
                println!("no results for '{}'", &options.query);
            } else {
                println!("Results");
                for (i, deviation) in results.deviations.iter().enumerate() {
                    println!("{}) {}", i + 1, deviation.title);
                    println!("Id: {}", deviation.deviation_id);
                    println!("Kind: {}", deviation.kind);
                    println!("Url: {}", deviation.url);
                    println!("Is downloadable: {}", deviation.is_downloadable);
                    println!();
                }
            }

            if !options.no_login {
                save_cookie_jar(&client).context("failed to save cookies")?;
            }
        }
        SubCommand::Download(options) => {
            let config = Config::load()
                .await
                .map(|config| {
                    println!("loaded config");
                    config
                })
                .unwrap_or_else(|e| {
                    println!("Failed to load config: {:?}", e);
                    Config::new()
                });
            println!();

            if !options.no_login {
                try_signin_cli(
                    &client,
                    config.username.as_deref(),
                    config.password.as_deref(),
                )
                .await?;
            }

            let scraped_webpage_info = client
                .scrape_webpage(&options.url)
                .await
                .context("failed to scrape webpage")?;
            let current_deviation = scraped_webpage_info
                .get_current_deviation()
                .context("failed to get current deviation")?;

            println!("Title: {}", current_deviation.title);
            println!("ID: {}", current_deviation.deviation_id);
            println!("Kind: {}", current_deviation.kind);
            println!("Url: {}", current_deviation.url);
            println!("Is downloadable: {}", current_deviation.is_downloadable);
            println!();

            if current_deviation.is_literature() {
                println!("Generating html page...");

                let text_content = current_deviation
                    .text_content
                    .as_ref()
                    .context("deviation is missing text content")?;
                let markup = text_content
                    .html
                    .get_markup()
                    .context("deviation is missing markup")?
                    .context("failed to parse markup");

                let filename = escape_path(&format!(
                    "{}-{}.html",
                    current_deviation.title, current_deviation.deviation_id
                ));

                if Path::new(&filename).exists() {
                    anyhow::bail!("file already exists");
                }

                let mut html = String::with_capacity(1_000_000); // 1 MB

                html.push_str("<html>");
                html.push_str("<head>");
                html.push_str("<meta charset=\"UTF-8\">");
                write!(&mut html, "<title>{}</title>", &current_deviation.title)?;
                html.push_str("<style>");
                html.push_str("html { font-family: devioussans02extrabold,Helvetica Neue,Helvetica,Arial,メイリオ, meiryo,ヒラギノ角ゴ pro w3,hiragino kaku gothic pro,sans-serif; }");
                html.push_str("body { background-color: #06070d; margin: 0; padding-bottom: 56px; padding-top: 56px; }");
                html.push_str("h1 { color: #f2f2f2; font-weight: 400; font-size: 48px; line-height: 1.22; letter-spacing: .3px;}");
                html.push_str("span { color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px; }");
                html.push_str("</style>");
                html.push_str("</head>");

                html.push_str("<body>");

                html.push_str("<div style=\"width:780px;margin:auto;\">");
                write!(&mut html, "<h1>{}</h1>", &current_deviation.title)?;

                match markup {
                    Ok(markup) => {
                        for block in markup.blocks.iter() {
                            write!(&mut html, "<div id = \"{}\">", block.key)?;

                            html.push_str("<span>");
                            if block.text.is_empty() {
                                html.push_str("<br>");
                            } else {
                                html.push_str(&block.text);
                            }
                            html.push_str("</span>");

                            html.push_str("</div>");
                        }
                    }
                    Err(e) => {
                        println!("Failed to parse markdown block format: {:?}", e);
                        println!("Interpeting as raw html...");

                        write!(&mut html, "<div style=\"color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px;\">{}</div>", text_content.html.markup.as_ref().context("missing markdown")?)?;
                    }
                }

                html.push_str("</div>");
                html.push_str("</body>");
                html.push_str("</html>");

                tokio::fs::write(filename, html).await?;
            } else if current_deviation.is_image() {
                println!("Downloading image...");
                let mut url = if !options.no_login {
                    scraped_webpage_info
                        .get_current_deviation_extended()
                        .and_then(|deviation_extended| deviation_extended.download.as_ref())
                        .map(|download| download.url.clone())
                } else {
                    current_deviation.get_download_url()
                };

                if url.is_none() && options.allow_fullview {
                    url = current_deviation.get_fullview_url();
                }

                let url = url.context("failed to select an image url")?;
                let extension = current_deviation
                    .media
                    .get_extension()
                    .context("could not determine image extension")?;

                let filename = escape_path(&format!(
                    "{}-{}.{}",
                    current_deviation.title, current_deviation.deviation_id, extension
                ));

                println!("Out Path: {}", filename);

                if Path::new(&filename).exists() {
                    anyhow::bail!("file already exists");
                }

                let bytes = client
                    .client
                    .get(url.as_str())
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;

                tokio::fs::write(filename, bytes).await?;
            } else {
                anyhow::bail!("unknown deviation type: {}", current_deviation.kind);
            }

            if !options.no_login {
                save_cookie_jar(&client).context("failed to save cookies")?;
            }
        }
    }

    Ok(())
}

async fn try_signin_cli(
    client: &deviantart::Client,
    username: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<()> {
    if let Err(e) = load_cookie_jar(&client) {
        eprintln!("Failed to load cookie jar: {}", e);
    }

    if !client
        .is_logged_in_online()
        .await
        .context("failed to check if logged in")?
    {
        match (username, password) {
            (Some(username), Some(password)) => {
                println!("logging in...");
                client
                    .signin(username, password)
                    .await
                    .context("failed to login")?;
                println!("logged in");
                println!();
            }
            (None, Some(_password)) => {
                anyhow::bail!("missing username");
            }
            (Some(_username), None) => {
                anyhow::bail!("missing password");
            }
            (None, None) => {
                anyhow::bail!("missing username and password");
            }
        }
    }

    Ok(())
}

fn get_cookie_file_path() -> anyhow::Result<PathBuf> {
    let base_dirs = directories_next::BaseDirs::new().context("failed to get base dirs")?;
    Ok(base_dirs.data_dir().join("deviantart/cookies.json"))
}

fn load_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    let cookie_file =
        std::fs::File::open(get_cookie_file_path()?).context("failed to read cookies")?;

    client
        .cookie_store
        .load_json(std::io::BufReader::new(cookie_file))?;

    Ok(())
}

fn save_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    let cookie_file =
        std::fs::File::create(get_cookie_file_path()?).context("failed to create cookie file")?;

    client.cookie_store.save_json(cookie_file)?;

    Ok(())
}

fn escape_path(path: &str) -> String {
    path.chars()
        .filter(|&c| c != ':' && c != '?' && c != '/')
        .collect()
}
