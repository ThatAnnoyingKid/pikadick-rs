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
    utils::Colour,
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

    info!("Reporting all cache stats");

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Cache Stats");
                e.color(Colour::from_rgb(255, 0, 0));

                for (stat_family_name, stat_family) in stats.iter() {
                    // Low ball, but better than nothing
                    let mut output = String::with_capacity(stats.len() * 16);

                    for (stat_name, stat) in stat_family.iter() {
                        writeln!(&mut output, "**{}**: {} item(s)", stat_name, stat).unwrap();
                    }

                    e.field(stat_family_name, &output, false);
                }

                e
            })
        })
        .await?;

    Ok(())
}
