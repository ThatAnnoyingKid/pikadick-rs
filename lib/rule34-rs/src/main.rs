use anyhow::Context;
use std::path::PathBuf;
use tokio::fs::File;

#[derive(argh::FromArgs)]
#[argh(description = "A utility to get rule34 images")]
pub struct Options {
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
#[argh(subcommand, name = "search", description = "search for a rule34 post")]
pub struct SearchOptions {
    #[argh(positional, description = "the query string")]
    query: String,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download", description = "download a rule34 post")]
pub struct DownloadOptions {
    #[argh(positional, description = "the post id")]
    id: u64,

    #[argh(
        option,
        short = 'o',
        default = "PathBuf::from(\".\")",
        description = "the path to save images"
    )]
    out_dir: PathBuf,
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
        .build()?;
    tokio_rt.block_on(async_main(options))?;
    println!("Done.");
    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = rule34::Client::new();

    match options.subcommand {
        SubCommand::Search(options) => {
            let results = client.search(&options.query).await?;

            if results.entries.is_empty() {
                println!("No Results");
            }

            for (i, result) in results.entries.iter().enumerate() {
                println!("{})", i + 1);
                println!("ID: {}", result.id);
                println!("Link: {}", result.link);
                println!("Description: {}", result.description);
                println!();
            }
        }
        SubCommand::Download(options) => {
            let post = client.get_post(options.id).await?;
            let image_name = post.get_image_name().context("missing image name")?;
            let out_path = options.out_dir.join(image_name);

            tokio::fs::create_dir_all(&options.out_dir)
                .await
                .context("failed to create out dir")?;

            println!("ID: {}", options.id);
            println!("Image Url: {}", post.image_url);
            println!("Image Name: {}", image_name);
            println!("Out Path: {}", out_path.display());
            println!();

            println!("Downloading...");
            let mut buffer = Vec::with_capacity(1_000_000); // 1 MB
            client
                .get_to(&post.image_url, &mut buffer)
                .await
                .context("failed to download image")?;

            let mut file = File::create(out_path).await?;
            tokio::io::copy(&mut buffer.as_slice(), &mut file)
                .await
                .context("failed to save image")?;
        }
    }

    Ok(())
}
