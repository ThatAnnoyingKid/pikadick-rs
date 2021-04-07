use anyhow::Context;
use open_graph::OpenGraphObject;

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

    download_object(&client, &object).await?;

    Ok(())
}

/// Download a url's contents
async fn download_object(
    client: &open_graph::Client,
    object: &OpenGraphObject,
) -> anyhow::Result<()> {
    let filename = if object.is_video() {
        object.get_video_url_file_name().unwrap_or("video.mp4")
    } else if object.is_image() {
        object.get_image_file_name().unwrap_or("image.png")
    } else {
        anyhow::bail!("Unsupported Object Kind '{}'", object.kind);
    };

    let mut buffer = Vec::with_capacity(1_000_000); // 1 MB
    client
        .download_object_to(&object, &mut buffer)
        .await
        .context("failed to download object")?;

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
