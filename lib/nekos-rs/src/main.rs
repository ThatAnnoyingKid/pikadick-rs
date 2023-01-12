use anyhow::{
    ensure,
    Context,
};
use std::path::PathBuf;

#[derive(argh::FromArgs)]
#[argh(description = "A utility to get random catgirl images")]
pub struct Command {
    #[argh(
        option,
        description = "whether the images whould be nsfw. defaults to both"
    )]
    nsfw: Option<bool>,

    #[argh(
        option,
        description = "the dir to output images",
        short = 'o',
        default = "PathBuf::from(\".\")"
    )]
    out_dir: PathBuf,
}

fn main() {
    let command: Command = argh::from_env();
    let code = match real_main(command) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };
    std::process::exit(code);
}

fn real_main(command: Command) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start tokio runtime")?;

    tokio_rt.block_on(async_main(command))
}

async fn async_main(command: Command) -> anyhow::Result<()> {
    let client = nekos::Client::new();

    let mut image_list = client.get_random(command.nsfw, 1).await?;
    ensure!(!image_list.images.is_empty(), "image list is empty");
    let image = image_list.images.swap_remove(0);

    tokio::fs::create_dir_all(&command.out_dir)
        .await
        .context("failed to create out dir")?;

    let filename = format!("{}.png", image.id);
    let current_dir = command.out_dir.join(filename);
    let image_url = image.get_url().context("failed to get url")?;

    println!("Url: {}", image_url.as_str());
    println!("Saving to: {}", current_dir.display());
    println!();

    nd_util::download_to_path(&client.client, image_url.as_str(), current_dir).await?;

    Ok(())
}
