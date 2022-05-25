use crate::checks::ENABLED_CHECK;
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

#[command]
#[only_in(guilds)]
#[bucket("default")]
#[checks(Enabled)]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg
        .guild_field(&ctx.cache, |guild| guild.id)
        .context("missing server data")?;

    let manager = songbird::get(ctx)
        .await
        .expect("missing songbird data")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();

        msg.channel_id.say(&ctx.http, "Playing song").await?;
    } else {
        msg.channel_id
            .say(&ctx.http, "Not in a voice channel")
            .await?;
    }

    Ok(())
}
