use anyhow::Context;
use open_graph::OpenGraphObject;
use std::path::Path;

#[derive(argh::FromArgs)]
#[argh(description = "a tool to download media from open-graph compatible sources")]
struct CommandOptions {
    #[argh(
        positional,
        description = "the url to a open graph protocol compatible webpage"
    )]
    url: String,

    #[argh(switch, description = "whether to print the debug open graph object")]
    debug_object: bool,
}

fn main() {
    let exit_code = {
        let options: CommandOptions = argh::from_env();
        let tokio_rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(tokio_rt) => tokio_rt,
            Err(e) => {
                eprintln!("Failed to start tokio runtime: {}", e);
                return;
            }
        };

        let ret = tokio_rt.block_on(async_main(options));

        match ret {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}", e);
                1
            }
        }
    };

    std::process::exit(exit_code);
}

async fn async_main(options: CommandOptions) -> anyhow::Result<()> {
    let client = open_graph::Client::new();

    let object = client
        .get_object(&options.url)
        .await
        .context("failed to get object")?;

    print_object(&object);
    println!();

    if options.debug_object {
        println!("{:#?}", object);
        println!();
    }

    if object.is_video() {
        let video_url = object
            .video_url
            .as_ref()
            .context("missing video url in object")?;

        let extension = match Path::new(video_url.path())
            .extension()
            .map(|extension| extension.to_str())
        {
            Some(Some(extension)) => extension,
            Some(None) => {
                eprintln!("Invalid extension, using 'mp4'");
                "mp4"
            }
            None => {
                eprintln!("Unknown extension, using 'mp4'");
                "mp4"
            }
        };

        let filename = format!("video.{}", extension);

        download(&client, video_url.as_str(), &filename).await?;

        return Ok(());
    }

    if object.kind == "instapp:photo" {
        let extension = match Path::new(object.image.path())
            .extension()
            .map(|extension| extension.to_str())
        {
            Some(Some(extension)) => extension,
            Some(None) => {
                eprintln!("Invalid extension, using 'png'");
                "png"
            }
            None => {
                eprintln!("Unknown extension, using 'png'");
                "png"
            }
        };

        let filename = format!("image.{}", extension);
        download(&client, object.image.as_str(), &filename).await?;
        return Ok(());
    }

    anyhow::bail!("Unsupported Object Kind");
}

/// Download a url's contents
async fn download(client: &open_graph::Client, url: &str, filename: &str) -> anyhow::Result<()> {
    let response = client
        .client
        .get(url)
        .send()
        .await
        .context("Failed to start download request")?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("invalid status code '{}'", status);
    }

    let data = response
        .bytes()
        .await
        .context("failed to download request body")?;

    std::fs::write(filename, data)
        .with_context(|| format!("failed to download to file '{}'", filename))?;

    Ok(())
}

fn print_object(object: &OpenGraphObject) {
    println!("Title: {}", object.title);
    println!("Kind: {}", object.kind);
    println!("Image: {}", object.image.as_str());

    if let Some(description) = object.description.as_ref() {
        println!("Description: {}", description);
    }
}
