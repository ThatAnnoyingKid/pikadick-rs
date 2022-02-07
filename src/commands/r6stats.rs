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
use anyhow::Context as _;
use r6stats::UserData;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::sync::Arc;
use tracing::{
    error,
    info,
};

#[derive(Clone, Default, Debug)]
pub struct R6StatsClient {
    client: r6stats::Client,
    search_cache: TimedCache<String, UserData>,
}

impl R6StatsClient {
    pub fn new() -> Self {
        Self {
            client: r6stats::Client::new(),
            search_cache: TimedCache::new(),
        }
    }

    /// Get stats
    pub async fn get_stats(
        &self,
        query: &str,
    ) -> Result<Option<Arc<TimedCacheEntry<UserData>>>, r6stats::Error> {
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
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.r6stats_client.clone();
    drop(data_lock);

    let name = args.trimmed().current().expect("missing name");

    info!("Getting r6 stats for '{}' using r6stats", name);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    match client.get_stats(name).await {
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

            error!("Failed to get r6 stats for '{}' using r6stats: {}", name, e);
        }
    }

    client.search_cache.trim();

    Ok(())
}

/// Options for r6stats
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct R6StatsOptions {
    /// The user name
    name: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("r6stats")
        .description("Get r6 stats for a user from r6stats")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("name")
                .description("The name of the user")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .required(true)
                .build()?,
        )
        .on_process(|ctx, interaction, args: R6StatsOptions| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("missing client data");
            let client = client_data.r6stats_client.clone();
            drop(data_lock);

            info!("getting r6 stats for '{}' using r6stats", args.name);

            let result = client.get_stats(&args.name).await;

            interaction
                .create_interaction_response(&ctx.http, |res| {
                    res.interaction_response_data(|res| match result {
                        Ok(Some(entry)) => res.create_embed(|e| {
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
                        }),
                        Ok(None) => res.content("No results"),
                        Err(e) => {
                            error!(
                                "failed to get r6 stats for '{}' using r6stats: {}",
                                args.name, e
                            );

                            res.content(format!("Failed to get stats: {}", e))
                        }
                    })
                })
                .await?;

            client.search_cache.trim();

            Ok(())
        })
        .build()
        .context("failed to build r6stats command")
}
