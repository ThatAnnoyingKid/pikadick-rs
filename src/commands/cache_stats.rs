use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateMessage,
    },
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::{
        colour::Colour,
        prelude::*,
    },
    prelude::*,
};
use std::fmt::Write;
use tracing::info;

#[command("cache-stats")]
#[description("Get cache usage stats")]
#[checks(Enabled)]
#[bucket("default")]
pub async fn cache_stats(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let stats = client_data.generate_cache_stats();
    drop(data_lock);

    info!("reporting all cache stats");

    let mut embed_builder = CreateEmbed::new()
        .title("Cache Stats")
        .color(Colour::from_rgb(255, 0, 0));
    for (stat_family_name, stat_family) in stats.into_iter() {
        // Low ball, but better than nothing
        let mut output = String::with_capacity(stat_family.len() * 16);

        for (stat_name, stat) in stat_family.iter() {
            writeln!(&mut output, "**{stat_name}**: {stat} item(s)").unwrap();
        }

        embed_builder = embed_builder.field(stat_family_name, output, false);
    }

    let message_builder = CreateMessage::new().embed(embed_builder);

    msg.channel_id
        .send_message(&ctx.http, message_builder)
        .await?;

    Ok(())
}
