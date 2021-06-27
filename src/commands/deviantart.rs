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
    Database,
};
use anyhow::Context as _;
use deviantart::SearchResults;
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
    time::Instant,
};
use tracing::{
    error,
    info,
};

const DATA_STORE_NAME: &str = "deviantart";
const COOKIE_KEY: &str = "cookie-store";

/// A caching deviantart client
#[derive(Clone, Debug)]
pub struct DeviantartClient {
    client: deviantart::Client,
    search_cache: TimedCache<String, SearchResults>,
}

impl DeviantartClient {
    /// Make a new [`DeviantartClient`].
    pub async fn new(db: &Database) -> anyhow::Result<Self> {
        use std::io::BufReader;

        let client = deviantart::Client::new();

        let store = db.get_store(DATA_STORE_NAME).await;
        let cookie_data: Option<Vec<u8>> = store
            .get(COOKIE_KEY)
            .await
            .context("failed to get cookie data")?;

        match cookie_data {
            Some(cookie_data) => {
                client
                    .cookie_store
                    .load_json(BufReader::new(cookie_data.as_slice()))?;
            }
            None => {
                info!("Could not load cookie data");
            }
        }

        Ok(DeviantartClient {
            client,
            search_cache: TimedCache::new(),
        })
    }

    /// Signs in if necessary
    pub async fn signin(
        &self,
        db: &Database,
        username: &str,
        password: &str,
    ) -> anyhow::Result<()> {
        if !self.client.is_logged_in_online().await? {
            info!("Re-signing in");
            self.client.signin(username, password).await?;

            // Store the new cookies
            let store = db.get_store(DATA_STORE_NAME).await;
            let cookie_store = self.client.cookie_store.clone();
            let cookie_data = tokio::task::spawn_blocking(move || {
                let mut cookie_data = Vec::with_capacity(1_000_000); // 1 MB
                cookie_store.save_json(&mut cookie_data)?;
                anyhow::Result::<_>::Ok(cookie_data)
            })
            .await??;
            store.put(COOKIE_KEY, cookie_data).await?;
        }

        Ok(())
    }

    /// Search for deviantart images with a cache.
    pub async fn search(
        &self,
        db: &Database,
        username: &str,
        password: &str,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<SearchResults>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let start = Instant::now();
        self.signin(db, username, password)
            .await
            .context("failed to log in to deviantart")?;

        let list = self
            .client
            .search(query, 1)
            .await
            .context("failed to search")?;
        self.search_cache.insert(String::from(query), list);
        let end = Instant::now();

        info!("Searched deviantart in {:?}", end - start);

        self.search_cache
            .get_if_fresh(query)
            .context("missing entry")
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
    let db = client_data.db.clone();
    let config = client_data.config.clone();
    drop(data_lock);

    let query = args.trimmed().quoted().current().expect("missing query");

    info!("Searching for '{}' on deviantart", query);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    match client
        .search(
            &db,
            &config.deviantart.username,
            &config.deviantart.password,
            query,
        )
        .await
    {
        Ok(entry) => {
            let data = entry.data();
            let choice = data
                .deviations
                .iter()
                .filter_map(|deviation| {
                    if deviation.is_image() {
                        Some(
                            deviation
                                .get_image_download_url()
                                .or_else(|| deviation.get_fullview_url()),
                        )
                    } else if deviation.is_film() {
                        Some(deviation.get_best_video_url().cloned())
                    } else {
                        None
                    }
                })
                .choose(&mut rand::thread_rng());

            if let Some(choice) = choice {
                if let Some(url) = choice {
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
