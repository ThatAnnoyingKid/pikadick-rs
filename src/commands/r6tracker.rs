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
    Platform,
    R6Error,
    SessionsData,
    StatusCode,
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

#[derive(Debug)]
pub struct Stats {
    pub profile: UserData,
    pub sessions: SessionsData,
}

#[derive(Clone, Default, Debug)]
pub struct R6TrackerClient {
    client: Arc<r6tracker::Client>,
    search_cache: TimedCache<String, Stats>,
}

impl R6TrackerClient {
    /// Make a new r6 client with caching
    pub fn new() -> Self {
        R6TrackerClient {
            client: Arc::new(r6tracker::Client::new()),
            search_cache: TimedCache::new(),
        }
    }

    /// Get r6 stats
    pub async fn get_stats(&self, query: &str) -> Result<Arc<TimedCacheEntry<Stats>>, R6Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let profile = self.client.get_profile(query, Platform::Pc).await?.data;
        let sessions = self.client.get_sessions(query, Platform::Pc).await?.data;
        let entry = Stats { profile, sessions };
        self.search_cache.insert(String::from(query), entry);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("Valid r6tracker user data"))
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
#[example("Kooklxs")]
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

    let name = args.trimmed().current().expect("name");

    info!(logger, "Getting r6 stats for '{}' using r6tracker", name);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_stats(&name).await {
        Ok(entry) => {
            loading.send_ok();
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        let data = entry.data();

                        e.title(name).image(data.profile.avatar_url());

                        if let Some(c) = data.profile.season_color_u32() {
                            e.color(c);
                        }

                        if let Some(kd) = data.profile.kd() {
                            e.field("Overall K/D", kd, true);
                        }

                        if let Some(wl) = data.profile.wl() {
                            e.field("Overall Win / Loss", wl / 100.0, true);
                        }

                        if let Some(mmr) = data.profile.current_mmr() {
                            e.field("MMR", mmr, true);
                        }

                        if let Some(season) = data.profile.get_latest_season() {
                            if let Some(wl) = season.wl() {
                                e.field("Ranked Win / Loss", wl / 100.0, true);
                            }
                        }

                        if let Some(thumb) = data.profile.current_mmr_image() {
                            e.thumbnail(thumb);
                        }

                        e
                    })
                })
                .await?;
        }
        Err(R6Error::InvalidStatus(StatusCode::NOT_FOUND)) => {
            msg.channel_id.say(&ctx.http, "No results").await?;
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
