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
use r6tracker::{
    Error as R6Error,
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

/// R6Tracker stats for a user
#[derive(Debug)]
pub struct Stats {
    overwolf_player: r6tracker::OverwolfPlayer,
    profile: r6tracker::UserData,
}

#[derive(Clone, Default, Debug)]
pub struct R6TrackerClient {
    client: r6tracker::Client,
    search_cache: TimedCache<String, Stats>,
}

impl R6TrackerClient {
    /// Make a new r6 client with caching
    pub fn new() -> Self {
        R6TrackerClient {
            client: Default::default(),
            search_cache: Default::default(),
        }
    }

    /// Get R6Tracker stats
    pub async fn get_stats(&self, query: &str) -> Result<Arc<TimedCacheEntry<Stats>>, R6Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let (overwolf_player, profile) = futures::future::join(
            self.client.get_overwolf_player(query),
            self.client.get_profile(query, r6tracker::Platform::Pc),
        )
        .await;
        let entry = Stats {
            overwolf_player: overwolf_player?.into_result()?,
            profile: profile?.into_result()?,
        };
        self.search_cache.insert(String::from(query), entry);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("Valid R6Tracker Stats"))
    }
}

impl CacheStatsProvider for R6TrackerClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "r6tracker",
            "search_cache",
            self.search_cache.len() as f32,
        );
    }
}

#[command]
#[description("Get r6 stats for a user from r6tracker")]
#[usage("<player>")]
#[example("KingGeorge")]
#[bucket("r6tracker")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
async fn r6tracker(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.r6tracker_client.clone();
    let logger = client_data.logger.clone();
    drop(data_lock);

    let name = args.trimmed().current().expect("Valid Name");

    info!(logger, "Getting r6 stats for '{}' using R6Tracker", name);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_stats(&name).await {
        Ok(entry) => {
            loading.send_ok();
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    let stats = entry.data();
                    m.embed(|e| {
                        // We mix overwolf and non-overwolf data to get what we want.

                        // New Overwolf Api
                        e.title(&stats.overwolf_player.name)
                            .image(&stats.overwolf_player.avatar);

                        if let Some(season) =
                            stats.overwolf_player.current_season_best_region.as_ref()
                        {
                            e.field("Current Rank", &season.rank_name, true)
                                .field("Current MMR", season.mmr, true)
                                .field("Seasonal Ranked K/D", format!("{:.2}", season.kd), true)
                                .field("Seasonal Ranked Win %", season.win_pct, true)
                                .field("Seasonal # of Ranked Matches", season.matches, true);
                        }

                        // Best Rank/MMR lifetime stats are bugged in Overwolf.
                        // It shows the max ending stats.
                        //
                        // Try manual calculation based on the Overwolf API Season stats,
                        // falling back to manual calculation based on the overlay stats,
                        // falling back to the Overwolf API lifetime value, which is bugged.
                        //
                        let max_overwolf_season = stats.overwolf_player.get_max_season();
                        let max_season = stats.profile.get_max_season();
                        let max_mmr = max_overwolf_season
                            .map(|season| season.max_mmr)
                            .or_else(|| max_season.and_then(|season| season.max_mmr()))
                            .unwrap_or(stats.overwolf_player.lifetime_stats.best_mmr.mmr);
                        let max_rank = max_overwolf_season
                            .map(|season| season.max_rank.rank_name.as_str())
                            .or_else(|| {
                                max_season
                                    .and_then(|season| season.max_rank())
                                    .map(|rank| rank.name())
                            })
                            .unwrap_or(&stats.overwolf_player.lifetime_stats.best_mmr.name);

                        e.field("Best MMR", max_mmr, true)
                            .field("Best Rank", max_rank, true)
                            .field(
                                "Lifetime K/D",
                                &stats.overwolf_player.lifetime_stats.kd,
                                true,
                            )
                            .field(
                                "Lifetime Win %",
                                &stats.overwolf_player.lifetime_stats.win_pct,
                                true,
                            );

                        // Old Non-Overwolf API

                        // Overwolf API does not send season colors
                        if let Some(c) = stats.profile.season_color_u32() {
                            e.color(c);
                        }

                        // Overwolf API does not send non-svg rank thumbnails
                        if let Some(thumb) = stats.profile.current_mmr_image() {
                            e.thumbnail(thumb);
                        }

                        e
                    })
                })
                .await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get r6tracker stats: {}", e))
                .await?;

            error!(
                logger,
                "Failed to get r6 stats for '{}', using r6tracker: {}", name, e
            );
        }
    }

    client.search_cache.trim();

    Ok(())
}
