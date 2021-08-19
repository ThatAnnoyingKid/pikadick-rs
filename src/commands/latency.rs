use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use tracing::warn;

#[command]
#[description("Get the bot's latency in this server")]
#[checks(Enabled)]
#[bucket("default")]
async fn latency(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let shard_manager = client_data.shard_manager.clone();
    drop(data_lock);

    let shard_id = ShardId(ctx.shard_id);

    let latency = {
        let manager = shard_manager.lock().await;
        let runners = manager.runners.lock().await;
        let maybe_shard = runners.get(&shard_id);
        maybe_shard.and_then(|shard| shard.latency)
    };

    match latency {
        Some(latency) => {
            msg.channel_id
                .say(&ctx.http, format!("Shard Latency: {:?}", latency))
                .await?;
        }
        None => {
            warn!("Failed to get latency for shard: {}", shard_id);
            msg.channel_id
                .say(&ctx.http, "Failed to get latency")
                .await?;
        }
    }
    Ok(())
}
