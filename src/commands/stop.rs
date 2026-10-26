use crate::{
    PoiseContext,
    PoiseError,
};
use serenity::model::prelude::GuildId;

#[poise::command(slash_command, description_localized("en-US", "Stop playing a song."))]
pub async fn stop(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let guild_id: GuildId = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            ctx.reply("Not in a guild.").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Missing songbird data")
        .clone();

    match manager.get(guild_id) {
        Some(handler_lock) => {
            let mut handler = handler_lock.lock().await;
            handler.stop();

            ctx.reply("Stopped.").await?;
        }
        None => {
            ctx.reply("Not in a voice channel.").await?;
        }
    }

    Ok(())
}
