use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
    ClientDataKey,
};
use anyhow::Context as _;
use serenity::{
    client::Context,
    framework::standard::{
        macros::*,
        Args,
        CommandResult,
    },
    model::prelude::*,
};

#[command]
#[description("Get a random post from a subreddit")]
#[bucket("default")]
#[min_args(1)]
#[max_args(1)]
#[usage("<subreddit_name>")]
#[example("dogpictures")]
#[checks(Enabled)]
async fn reddit(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let reddit_embed_data = client_data.reddit_embed_data.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let subreddit = args.single::<String>().expect("missing arg");
    match reddit_embed_data
        .get_random_post(&subreddit)
        .await
        .context("failed fetching posts")
    {
        Ok(Some(url)) => {
            msg.channel_id.say(&ctx.http, url).await?;
            loading.send_ok();
        }
        Ok(None) => {
            msg.channel_id.say(&ctx.http, "No posts found").await?;
        }
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
        }
    }

    Ok(())
}
