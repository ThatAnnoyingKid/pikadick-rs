use crate::checks::ENABLED_CHECK;
use anyhow::Context as _;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        CommandResult,
    },
    model::prelude::*,
};

#[command]
#[only_in(guilds)]
#[bucket("default")]
#[checks(Enabled)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let (guild_id, channel_id) = msg
        .guild_field(&ctx.cache, |guild| {
            (
                guild.id,
                guild
                    .voice_states
                    .get(&msg.author.id)
                    .and_then(|voice_state| voice_state.channel_id),
            )
        })
        .await
        .context("missing server data")?;

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, "Not in a voice channel").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("missing songbird data")
        .clone();

    let _handler = manager.join(guild_id, connect_to).await;

    Ok(())
}
