use anyhow::Context;
use std::num::NonZeroU64;

#[derive(argh::FromArgs)]
#[argh(
    subcommand,
    name = "deleted",
    description = "get a list of deleted images"
)]
pub struct Options {
    #[argh(positional, description = "the start id of the posts retrieved")]
    last_id: Option<NonZeroU64>,
}

pub async fn exec(client: &rule34::Client, options: Options) -> anyhow::Result<()> {
    let list = client
        .list_deleted_images(options.last_id)
        .await
        .context("failed to get deleted images")?;

    for (i, post) in list.posts.iter().enumerate() {
        println!("{})", i + 1);
        println!("ID: {}", post.deleted);
        if let Some(md5) = post.md5 {
            println!("MD5: {md5}");
        }
        println!();
    }

    Ok(())
}
