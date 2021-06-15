use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use tracing::error;

#[command]
#[description("Get a random comic from Xkcd")]
#[checks(Enabled)]
async fn xkcd(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.xkcd_client.clone();
    drop(data_lock);

    match client.get_random().await {
        Ok(data) => {
            msg.channel_id.say(&ctx.http, data).await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get xkcd comic: {}", e))
                .await?;
            error!("Failed to get xkcd comic: {}", e);
        }
    }

    Ok(())
}
