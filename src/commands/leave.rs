use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
};
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
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg
        .guild_field(&ctx.cache, |guild| guild.id)
        .await
        .context("missing server data")?;

    let mut loading_reaction = LoadingReaction::new(ctx.http.clone(), msg);

    let manager = songbird::get(ctx)
        .await
        .expect("missing songbird data")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        match manager.remove(guild_id).await {
            Ok(()) => loading_reaction.send_ok(),
            Err(e) => {
                msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
            }
        }
    } else {
        msg.reply(ctx, "Not in a voice channel").await?;
    }

    Ok(())
}
