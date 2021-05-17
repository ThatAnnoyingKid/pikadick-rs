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
use deviantart::SearchResults;
use log::{
    error,
    info,
};
use parking_lot::Mutex;
use rand::seq::IteratorRandom;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

/// A caching deviantart client
#[derive(Clone, Default, Debug)]
pub struct DeviantartClient {
    client: deviantart::Client,
    search_cache: TimedCache<String, SearchResults>,
    last_update: Arc<Mutex<Option<Instant>>>,
}

impl DeviantartClient {
    /// Make a new [`DeviantartClient`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Signs in if necessary
    pub async fn signin(&self, username: &str, password: &str) -> Result<(), deviantart::Error> {
        let do_update = {
            let last_update = self.last_update.lock();
            last_update.map_or(true, |last_update| {
                Instant::elapsed(&last_update) > Duration::from_secs(60 * 30)
            })
        };

        if do_update {
            info!("Re-signing in");

            self.client.signin(username, password).await?;
            *self.last_update.lock() = Some(Instant::now());
        }

        Ok(())
    }

    /// Search for deviantart images with a cache.
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Arc<TimedCacheEntry<SearchResults>>, deviantart::Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let list = self.client.search(query, 1).await?;
        self.search_cache.insert(String::from(query), list);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("invalid entry"))
    }
}

impl CacheStatsProvider for DeviantartClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "deviantart",
            "search_cache",
            self.search_cache.len() as f32,
        );
    }
}

#[command]
#[description("Get art from deviantart")]
#[usage("<query>")]
#[example("sun")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
async fn deviantart(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing clientdata");
    let client = client_data.deviantart_client.clone();
    let config = client_data.config.clone();
    drop(data_lock);

    let query = args.trimmed().quoted().current().expect("missing query");

    info!("Searching for '{}' on deviantart", query);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    if let Err(e) = client
        .signin(&config.deviantart.username, &config.deviantart.password)
        .await
    {
        error!("Failed to log into deviantart: {}", e);
        msg.channel_id
            .say(&ctx.http, "Failed to log in to deviantart")
            .await?;
    }

    match client.search(&query).await {
        Ok(entry) => {
            let data = entry.data();
            let choice = data
                .deviations
                .iter()
                .filter(|d| d.is_image())
                .choose(&mut rand::thread_rng());

            if let Some(choice) = choice {
                if let Some(url) = choice
                    .get_download_url()
                    .or_else(|| choice.get_fullview_url())
                    .or_else(|| choice.get_gif_url())
                {
                    loading.send_ok();
                    msg.channel_id.say(&ctx.http, url).await?;
                } else {
                    msg.channel_id
                        .say(&ctx.http, "Missing url. This is probably a bug.")
                        .await?;
                    error!("DeviantArt deviation missing asset url: {:?}", choice);
                }
            } else {
                msg.channel_id.say(&ctx.http, "No Results").await?;
            }
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to search '{}': {:?}", query, e))
                .await?;

            error!("Failed to search for {} on deviantart: {:?}", query, e);
        }
    }

    client.search_cache.trim();

    Ok(())
}
