use anyhow::Context;
use std::{
    collections::{
        HashSet,
        VecDeque,
    },
    convert::TryFrom,
    path::{
        Path,
        PathBuf,
    },
};
use tokio::{
    fs::File,
    io::{
        AsyncWriteExt,
        BufWriter,
    },
};

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
        long = "download-children",
        description = "whether to download child posts"
    )]
    download_children: bool,

    #[argh(
        switch,
        long = "download-parent",
        description = "whether to download parent posts"
    )]
    download_parent: bool,

    #[argh(
        switch,
        short = 'd',
        long = "dry-run",
        description = "whether to save the image"
    )]
    dry_run: bool,
}

pub async fn exec(client: &rule34::Client, options: Options) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(&options.out_dir)
        .await
        .context("failed to create out dir")?;

    let mut downloaded = HashSet::with_capacity(8);
    let mut queue = VecDeque::with_capacity(8);
    queue.push_back(options.id);

    while let Some(id) = queue.pop_front() {
        let post = client.get_post(id).await.context("failed to get post")?;
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

        print_post_info(&post, image_name, &out_path);

        downloaded.insert(post.id);

        if out_path.exists() {
            println!("file already exists");
        } else if options.dry_run {
            println!("Not saving since this is a dry run...")
        } else {
            println!("Downloading...");
            let mut file = BufWriter::new(File::create(out_path).await?);
            client
                .get_to_writer(post.image_url.as_str(), &mut file)
                .await
                .context("failed to download image")?;

            file.flush().await.context("failed to flush writer")?;
        }

        if options.download_parent {
            if let Some(id) = post.parent_post {
                if !downloaded.contains(&id) {
                    queue.push_back(id);
                }
            }
        }

        if options.download_children && post.has_child_posts {
            let mut results = Vec::with_capacity(64);
            let mut offset = 0;
            loop {
                let search_query = format!("parent:{}", post.id);
                let page_results = client
                    .search(&search_query, offset)
                    .await
                    .context("failed to fetch post children")?
                    .entries;
                offset += u64::try_from(page_results.len())
                    .context("failed to convert the page results to a usize")?;
                if page_results.is_empty() {
                    break;
                }
                results.extend(page_results);
            }

            for result in results {
                if !downloaded.contains(&result.id) {
                    queue.push_back(result.id);
                }
            }
        }
    }

    Ok(())
}

fn print_post_info(post: &rule34::Post, image_name: &str, out_path: &Path) {
    println!("ID: {}", post.id);
    println!("Post Date: {}", post.date);
    println!("Post Url: {}", post.get_post_url());
    if let Some(source) = post.source.as_ref() {
        println!("Post Source: {}", source);
    }
    println!("Image Url: {}", post.image_url);
    println!("Image Name: {}", image_name);
    println!("Copyright Tags: {}", post.copyright_tags.join(", "));
    println!("Character Tags: {}", post.character_tags.join(", "));
    println!("Artist Tags: {}", post.artist_tags.join(", "));
    println!("General Tags: {}", post.general_tags.join(", "));
    println!("Meta Tags: {}", post.meta_tags.join(", "));
    println!("Has Child Posts: {}", post.has_child_posts);
    println!(
        "Parent Post: {}",
        post.parent_post
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!("Out Path: {}", out_path.display());
    println!();
}
