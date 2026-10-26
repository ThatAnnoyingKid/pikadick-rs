use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use serenity::model::prelude::GuildId;
use std::fmt::Write;
use tracing::warn;

#[poise::command(
    slash_command,
    description_localized("en-US", "Leave the voice channel."),
    check = "crate::checks::enabled"
)]
pub async fn leave(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
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
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        let mut content = String::new();

        if let Err(error) = manager
            .remove(guild_id)
            .await
            .context("Failed to remove guild")
        {
            warn!("{error:?}");

            writeln!(&mut content, "{error:?}")?;
        }
        writeln!(&mut content)?;
        writeln!(&mut content, "Left voice channel.")?;

        ctx.reply(content).await?;
    } else {
        ctx.reply("Not in a voice channel.").await?;
    }

    Ok(())
}
