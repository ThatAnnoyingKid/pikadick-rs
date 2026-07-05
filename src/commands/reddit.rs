use crate::{
    ClientDataKey,
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use tracing::error;

#[poise::command(
    slash_command,
    description_localized("en-US", "Get a random post from a subreddit"),
    check = "crate::checks::enabled"
)]
pub async fn reddit(
    ctx: PoiseContext<'_>,
    #[description = "The name of the subreddit"] subreddit: String,
) -> Result<(), PoiseError> {
    let data_lock = ctx.serenity_context().data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let reddit_embed_data = client_data.reddit_embed_data.clone();
    drop(data_lock);

    ctx.defer().await?;

    let content = match reddit_embed_data
        .get_random_post(&subreddit)
        .await
        .context("failed fetching posts")
    {
        Ok(Some(url)) => url,
        Ok(None) => "No posts found".into(),
        Err(error) => {
            error!("{error:?}");
            format!("{error:?}")
        }
    };
    ctx.reply(content).await?;

    Ok(())
}
