use anyhow::Context;
use std::path::{
    Path,
    PathBuf,
};
use tokio::fs::File;

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "download", description = "download a rule34 post")]
pub struct Options {
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

pub async fn exec(client: &rule34::Client, options: Options) -> anyhow::Result<()> {
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

    Ok(())
}
