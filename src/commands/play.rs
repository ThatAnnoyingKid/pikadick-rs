use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
};
use anyhow::Context as _;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
};
use tracing::error;

#[command]
#[only_in(guilds)]
#[min_args(1)]
#[bucket("default")]
#[checks(Enabled)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>().expect("missing url");
    if !url.starts_with("http") {
        msg.channel_id.say(&ctx.http, "invalid url").await?;
        return Ok(());
    }

    let mut loading_reaction = LoadingReaction::new(ctx.http.clone(), msg);

    let guild_id = msg
        .guild_field(&ctx.cache, |guild| guild.id)
        .await
        .context("missing server data")?;

    let manager = songbird::get(ctx)
        .await
        .expect("missing songbird data")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match songbird::ytdl(&url).await.context("failed to stream") {
            Ok(source) => source,
            Err(e) => {
                error!("{:?}", e);
                msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
                return Ok(());
            }
        };

        handler.play_only_source(source);
        loading_reaction.send_ok();
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel")
            .await?;
    }

    Ok(())
}
