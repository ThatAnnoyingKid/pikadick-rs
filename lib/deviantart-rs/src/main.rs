use anyhow::Context;
use mime_guess::Mime;
use std::fmt::Write;

#[derive(argh::FromArgs)]
#[argh(description = "a tool to interact with deviantart")]
struct Options {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    Search(SearchOptions),
    Download(DownloadOptions),
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "search")]
#[argh(description = "search on deviantart")]
struct SearchOptions {
    #[argh(positional, description = "the query string")]
    query: String,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download")]
#[argh(description = "download from deviantart")]
struct DownloadOptions {
    #[argh(positional, description = "the deviation url")]
    url: String,
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
        SubCommand::Search(options) => {
            let results = client.search(&options.query).await?;

            for (i, deviation) in results.deviations.iter().enumerate() {
                println!("{}) {}", i + 1, deviation.title,);
                println!("Id: {}", deviation.deviation_id);
                println!("Kind: {}", deviation.kind);
                println!("Url: {}", deviation.url);
                println!();
            }
        }
        SubCommand::Download(options) => {
            let scraped_deviation_info = client
                .scrape_deviation(&options.url)
                .await
                .context("failed to scrape deviation")?;
            let current_deviation = scraped_deviation_info
                .get_current_deviation()
                .context("failed to get current deviation")?;

            println!("Title: {}", current_deviation.title);
            println!("ID: {}", current_deviation.deviation_id);
            println!("Type: {}", current_deviation.kind);
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
                    .context("failed to parse markup")?;

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

                html.push_str("</div>");
                html.push_str("</body>");
                html.push_str("</html>");

                tokio::fs::write(
                    format!(
                        "{}-{}.html",
                        current_deviation.title, current_deviation.deviation_id
                    ),
                    html,
                )
                .await?;
            } else if current_deviation.is_image() {
                println!("Downloading image...");
                let url = current_deviation
                    .get_download_url()
                    .context("missing download url")?;
                let (bytes, mime) = {
                    let res = client
                        .client
                        .get(url.as_str())
                        .send()
                        .await?
                        .error_for_status()?;
                    let mime: Mime = res
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .context("missing content type header")?
                        .to_str()
                        .context("invalid content type header")?
                        .parse()
                        .context("failed to parse content type header")?;
                    let bytes = res.bytes().await?;

                    (bytes, mime)
                };

                let extension = mime_guess::get_mime_extensions(&mime)
                    .and_then(|extensions| extensions.last())
                    .with_context(|| format!("missing mime extensions for {}", mime))?;
                tokio::fs::write(
                    format!(
                        "{}-{}.{}",
                        current_deviation.title, current_deviation.deviation_id, extension
                    ),
                    bytes,
                )
                .await?;
            } else {
                anyhow::bail!("unknown deviation type: {}", current_deviation.kind);
            }
        }
    }

    Ok(())
}
