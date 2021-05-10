use anyhow::Context;
use std::path::PathBuf;
use tokio::fs::File;

#[derive(argh::FromArgs)]
#[argh(description = "A utility to get rule34 images")]
pub struct Command {
    #[argh(positional)]
    #[argh(description = "the query string of the to-be-downloaded image")]
    query: String,

    #[argh(option, short = 'o', default = "PathBuf::from(\".\")")]
    #[argh(description = "the path to save images")]
    out_dir: PathBuf,
}

fn main() {
    let exit_code = {
        let command: Command = argh::from_env();
        if let Err(e) = real_main(command) {
            eprintln!("{:?}", e);
            1
        } else {
            0
        }
    };

    std::process::exit(exit_code);
}

fn real_main(command: Command) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    tokio_rt.block_on(async_main(command))
}

async fn async_main(command: Command) -> anyhow::Result<()> {
    let client = rule34::Client::new();
    let results = client.search(&command.query).await?;

    let first_result = results.entries.get(0).context("no results")?;

    let post = client.get_post(first_result.id).await?;

    let image_name = post.get_image_name().context("missing image name")?;
    let current_dir = command.out_dir.join(image_name);
    tokio::fs::create_dir_all(&command.out_dir).await?;

    println!("ID: {}", first_result.id);
    println!("Image Url: {}", post.image_url);
    println!("Image Name: {}", image_name);
    println!("Out Dir: {}", current_dir.display());
    println!();

    println!("Downloading...");
    let mut file = File::create(current_dir).await?;
    client.get_to(&post.image_url, &mut file).await?;
    println!("Done.");

    Ok(())
}
