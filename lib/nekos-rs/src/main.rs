use std::{
    fs::File,
    path::PathBuf,
};

fn default_out_dir() -> PathBuf {
    ".".into()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Nekos(#[from] nekos::NekosError),

    #[error("{0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("the api sent no images")]
    MissingImages,
}

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
        default = "default_out_dir()"
    )]
    out_dir: PathBuf,
}

fn main() -> Result<(), Error> {
    let command: Command = argh::from_env();
    let mut tokio_rt = tokio::runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .build()?;
    tokio_rt.block_on(async_main(command))
}

async fn async_main(command: Command) -> Result<(), Error> {
    let client = nekos::Client::new();

    let mut image_list = client.get_random(command.nsfw, 1).await?;

    if image_list.images.is_empty() {
        eprintln!("Missing random images");
        return Err(Error::MissingImages);
    }

    let image = image_list.images.swap_remove(0);

    std::fs::create_dir_all(&command.out_dir)?;
    let filename = format!("{}.png", image.id);
    let current_dir = command.out_dir.join(filename);
    let image_url = image.get_url()?;

    println!("Url: {}", image_url.as_str());
    println!("Saving to: {}", current_dir.display());
    println!();

    let file = File::create(current_dir)?;
    client.copy_res_to(&image_url, file).await?;

    Ok(())
}
