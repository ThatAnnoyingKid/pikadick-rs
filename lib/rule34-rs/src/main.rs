use std::{
    fs::File,
    path::PathBuf,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Rule34(#[from] rule34::RuleError),

    #[error("no results")]
    NoResults,

    #[error("missing image name")]
    MissingImageName,
}

fn default_out_dir() -> PathBuf {
    ".".into()
}

#[derive(argh::FromArgs)]
#[argh(description = "A utility to get rule34 images")]
pub struct Command {
    #[argh(positional)]
    #[argh(description = "the query string of the to-be-downloaded image")]
    query: String,

    #[argh(option, short = 'o', default = "default_out_dir()")]
    #[argh(description = "the path to save images")]
    out_dir: PathBuf,
}

fn main() -> Result<(), Error> {
    let command: Command = argh::from_env();
    let mut tokio_rt = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()?;

    tokio_rt.block_on(async_main(command))
}

async fn async_main(command: Command) -> Result<(), Error> {
    let client = rule34::Client::new();
    let results = client.search(&command.query).await?;

    let first_result = results
        .entries
        .iter()
        .next()
        .and_then(|o| o.as_ref())
        .ok_or(Error::NoResults)?;

    let post = client.get_post(first_result.id).await?;

    let image_name = post.get_image_name().ok_or(Error::MissingImageName)?;
    let current_dir = command.out_dir.join(image_name);
    std::fs::create_dir_all(&command.out_dir)?;

    println!("ID: {}", first_result.id);
    println!("Image Url: {}", post.image_url);
    println!("Image Name: {}", image_name);
    println!("Out Dir: {}", current_dir.display());
    println!();

    println!("Downloading...");
    let mut file = File::create(current_dir)?;
    client.copy_res_to(&post.image_url, &mut file).await?;
    println!("Done.");

    Ok(())
}
