use anyhow::Context;
use std::path::{
    Path,
    PathBuf,
};
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

    #[argh(
        option,
        long = "offset",
        default = "0",
        description = "the starting offset"
    )]
    offset: u64,
}

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download", description = "download a rule34 post")]
pub struct DownloadOptions {
    #[argh(positional, description = "the post id")]
    id: u64,

    #[argh(
        option,
        short = 'o',
        long = "out-dir",
        default = "PathBuf::from(\".\")",
        description = "the path to save images"
    )]
    out_dir: PathBuf,

    #[argh(
        switch,
        short = 'd',
        long = "dry-run",
        description = "whether to save the image"
    )]
    dry_run: bool,
}

fn main() {
    let options: Options = argh::from_env();
    let exit_code = {
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
            let results = client.search(&options.query, options.offset).await?;

            if results.entries.is_empty() {
                println!("No Results");
            }

            for (i, result) in results.entries.iter().enumerate() {
                println!("{})", i + 1);
                println!("ID: {}", result.id);
                println!("Url: {}", result.get_post_url());
                println!("Description: {}", result.description);
                println!();
            }
        }
        SubCommand::Download(options) => {
            let post = client
                .get_post(options.id)
                .await
                .context("failed to get post")?;
            let image_name = post.get_image_name().context("missing image name")?;
            let image_extension = Path::new(image_name)
                .extension()
                .context("missing image extension")?
                .to_str()
                .context("image extension is not valid unicode")?;

            let mut file_name_buffer = itoa::Buffer::new();
            let file_name = file_name_buffer.format(options.id);
            let out_path = options
                .out_dir
                .join(format!("{}.{}", file_name, image_extension));

            tokio::fs::create_dir_all(&options.out_dir)
                .await
                .context("failed to create out dir")?;

            println!("ID: {}", post.id);
            println!("Post Date: {}", post.date);
            println!("Post Url: {}", post.get_post_url());
            if let Some(source) = post.source.as_ref() {
                println!("Post Source: {}", source);
            }
            println!("Image Url: {}", post.image_url);
            println!("Image Name: {}", image_name);
            println!("Out Path: {}", out_path.display());
            println!();

            if out_path.exists() {
                anyhow::bail!("file already exists");
            }

            println!("Downloading...");
            let buffer = client
                .get_bytes(post.image_url.as_str())
                .await
                .context("failed to download image")?;

            if options.dry_run {
                println!("Not saving since this is a dry run...")
            } else {
                println!("Saving...");
                let mut file = File::create(out_path).await?;
                tokio::io::copy(&mut buffer.as_ref(), &mut file)
                    .await
                    .context("failed to save image")?;
            }
        }
    }

    Ok(())
}
