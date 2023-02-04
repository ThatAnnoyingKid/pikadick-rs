#![allow(clippy::uninlined_format_args)]

use anyhow::{
    ensure,
    Context,
};
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
};
use url::Url;

#[derive(argh::FromArgs)]
#[argh(description = "A small CLI to download tiktok videos")]
struct CommandOptions {
    /// the post url
    #[argh(positional)]
    url: Url,

    /// the outfile
    #[argh(option, short = 'o', default = "PathBuf::from(\"video.mp4\")")]
    out_file: PathBuf,
}

fn main() {
    let options: CommandOptions = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            1
        }
    };

    std::process::exit(code);
}

fn real_main(options: CommandOptions) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    tokio_rt.block_on(async_main(options))?;

    Ok(())
}

async fn async_main(options: CommandOptions) -> anyhow::Result<()> {
    let client = tiktok::Client::new();

    eprintln!("Fetching post page...");
    let post = client
        .get_post(options.url.as_str())
        .await
        .context("failed to get post")?;
    let video_url = post.get_video_download_url().context("missing video url")?;

    eprintln!("Downloading video from '{}'", video_url.as_str());
    let mut file = File::create(&options.out_file)
        .await
        .context("failed to create file")?;

    download_to_file(
        &client.client,
        &mut file,
        video_url.as_str(),
        "https://www.tiktok.com/",
    )
    .await
    .context("failed to download video")?;

    Ok(())
}

/// Download a url to a file
async fn download_to_file(
    client: &reqwest::Client,
    file: &mut File,
    url: &str,
    referer: &str,
) -> anyhow::Result<()> {
    let mut response = client
        .get(url)
        .header(reqwest::header::REFERER, referer)
        .send()
        .await?
        .error_for_status()?;

    let content_length = response.content_length();
    if let Some(content_length) = content_length {
        file.set_len(content_length).await?;
    }

    let mut actual_length = 0;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        actual_length += u64::try_from(chunk.len())?;
    }

    if let Some(content_length) = content_length {
        ensure!(
            content_length == actual_length,
            "content-length header mismatch"
        );
    }

    Ok(())
}
