use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        LoadingReaction,
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
};
use r6stats::{
    Error as R6Error,
    UserData,
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
use slog::{
    error,
    info,
};
use std::sync::Arc;

#[derive(Clone, Default, Debug)]
pub struct R6StatsClient {
    client: r6stats::Client,
    search_cache: TimedCache<String, UserData>,
}

impl R6StatsClient {
    pub fn new() -> Self {
        Default::default()
    }

    /// Get stats
    pub async fn get_stats(
        &self,
        query: &str,
    ) -> Result<Option<Arc<TimedCacheEntry<UserData>>>, R6Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(Some(entry));
        }

        let mut user_list = self.client.search(query).await?;

        if user_list.is_empty() {
            return Ok(None);
        }

        let user = user_list.swap_remove(0);

        self.search_cache.insert(String::from(query), user);

        Ok(self.search_cache.get_if_fresh(query))
    }
}

impl CacheStatsProvider for R6StatsClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("r6stats", "search_cache", self.search_cache.len() as f32);
    }
}

#[command]
#[description("Get r6 stats for a user from r6stats")]
#[usage("<player>")]
#[example("KingGeorge")]
#[bucket("r6stats")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
async fn r6stats(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.r6stats_client.clone();
    let logger = client_data.logger.clone();
    drop(data_lock);

    let name = args.trimmed().current().unwrap();

    info!(logger, "Getting r6 stats for '{}' using r6stats", name);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_stats(&name).await {
        Ok(Some(entry)) => {
            loading.send_ok();

            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        let data = entry.data();

                        e.title(&data.username).image(data.avatar_url_256.as_str());

                        if let Some(kd) = data.kd() {
                            e.field("Overall Kill / Death", kd, true);
                        }

                        if let Some(wl) = data.wl() {
                            e.field("Overall Win / Loss", wl, true);
                        }

                        if let Some(stats) = data.seasonal_stats.as_ref() {
                            e.field("MMR", stats.mmr, true);
                            e.field("Max MMR", stats.max_mmr, true);
                            e.field("Mean Skill", stats.skill_mean, true);
                        }

                        e
                    })
                })
                .await?;
        }
        Ok(None) => {
            msg.channel_id.say(&ctx.http, "No results").await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get stats: {}", e))
                .await?;

            error!(
                logger,
                "Failed to get r6 stats for '{}' using r6stats: {}", name, e
            );
        }
    }

    client.search_cache.trim();

    Ok(())
}
