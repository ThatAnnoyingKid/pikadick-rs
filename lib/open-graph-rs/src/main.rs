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

        download(&client, video_url.as_str(), "mp4", "video").await?;

        return Ok(());
    }

    if object.kind == "instapp:photo" {
        download(&client, object.image.as_str(), "png", "image").await?;
        return Ok(());
    }

    anyhow::bail!("Unsupported Object Kind");
}

/// Download a url's contents
async fn download(
    client: &open_graph::Client,
    url: &str,
    default_extension: &str,
    filename: &str,
) -> anyhow::Result<()> {
    let extension = match Path::new(url)
        .extension()
        .map(|extension| extension.to_str())
    {
        Some(Some(extension)) => extension,
        Some(None) => {
            eprintln!("Invalid extension, using '{}'", default_extension);
            default_extension
        }
        None => {
            eprintln!("Unknown extension, using '{}'", default_extension);
            default_extension
        }
    };

    let filename = format!("{}.{}", filename, extension);

    let mut buffer = Vec::with_capacity(1_000_000); // 1 MB
    client
        .get_and_copy_to(url, &mut buffer)
        .await
        .with_context(|| format!("failed to download '{}'", url))?;

    std::fs::write(&filename, buffer)
        .with_context(|| format!("failed to download to file '{}'", filename))?;

    Ok(())
}

/// Pretty-print an [`OpenGraphObject`].
fn print_object(object: &OpenGraphObject) {
    println!("Title: {}", object.title);
    println!("Kind: {}", object.kind);
    println!("Image: {}", object.image.as_str());

    if let Some(description) = object.description.as_ref() {
        println!("Description: {}", description);
    }
}
