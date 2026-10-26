use crate::{
    PoiseContext,
    PoiseError,
};
use tracing::warn;

#[poise::command(
    slash_command,
    description_localized("en-US", "Get the bot's latency in this server."),
    check = "crate::checks::enabled"
)]
pub async fn latency(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let shard_manager = ctx.framework().shard_manager.clone();
    let shard_id = ctx.serenity_context().shard_id;

    let latency = {
        let runners = shard_manager.runners.lock().await;
        let maybe_shard = runners.get(&shard_id);
        maybe_shard.and_then(|shard| shard.latency)
    };

    match latency {
        Some(latency) => {
            ctx.reply(format!("Shard Latency: {latency:?}")).await?;
        }
        None => {
            warn!("Failed to get latency for shard: {shard_id}");
            ctx.reply("Failed to get latency.").await?;
        }
    }

    Ok(())
}
