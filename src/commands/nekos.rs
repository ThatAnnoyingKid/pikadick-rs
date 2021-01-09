use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::LoadingReaction,
    ClientDataKey,
};
use crossbeam::queue::ArrayQueue;
use indexmap::set::IndexSet;
use nekos::NekosError;
use parking_lot::RwLock;
use rand::Rng;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use slog::error;
use std::{
    str::FromStr,
    sync::Arc,
};
use url::Url;

/// Max images per single request
const BUFFER_SIZE: u8 = 100;

#[derive(Debug)]
struct NsfwArgParseError;

struct NsfwArg;

impl FromStr for NsfwArg {
    type Err = NsfwArgParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "nsfw" {
            Ok(NsfwArg)
        } else {
            Err(NsfwArgParseError)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cache(Arc<CacheInner>);

impl Cache {
    pub fn new() -> Self {
        Self(Arc::new(CacheInner {
            primary: ArrayQueue::new(BUFFER_SIZE.into()),
            secondary: RwLock::new(IndexSet::new()),
        }))
    }

    pub fn primary_len(&self) -> usize {
        self.0.primary.len()
    }

    pub fn secondary_len(&self) -> usize {
        self.0.secondary.read().len()
    }

    pub fn primary_is_empty(&self) -> bool {
        self.0.primary.is_empty()
    }

    pub fn secondary_is_empty(&self) -> bool {
        self.0.secondary.read().is_empty()
    }

    pub fn add(&self, uri: Url) {
        let _ = self.0.primary.push(uri.clone());
        self.0.secondary.write().insert(uri);
    }

    pub fn add_many<I: Iterator<Item = Url>>(&self, iter: I) {
        let mut secondary = self.0.secondary.write();
        for uri in iter {
            let _ = self.0.primary.push(uri.clone());
            secondary.insert(uri);
        }
    }

    pub async fn get_rand(&self) -> Option<Url> {
        if let Ok(uri) = self.0.primary.pop() {
            Some(uri)
        } else {
            let lock = self.0.secondary.read();

            if lock.is_empty() {
                return None;
            }

            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..lock.len());

            lock.get_index(index).cloned()
        }
    }
}

impl Default for Cache {
    fn default() -> Self {
        Cache::new()
    }
}

#[derive(Debug)]
struct CacheInner {
    primary: ArrayQueue<Url>,
    secondary: RwLock<IndexSet<Url>>,
}

#[derive(Clone, Debug)]
pub struct NekosClient {
    client: nekos::Client,

    cache: Cache,
    nsfw_cache: Cache,
}

impl NekosClient {
    pub fn new() -> Self {
        NekosClient {
            client: Default::default(),
            cache: Cache::new(),
            nsfw_cache: Cache::new(),
        }
    }

    fn get_cache(&self, nsfw: bool) -> &Cache {
        if nsfw {
            &self.nsfw_cache
        } else {
            &self.cache
        }
    }

    pub async fn populate(&self, nsfw: bool) -> Result<(), NekosError> {
        let cache = self.get_cache(nsfw);
        let image_list = self.client.get_random(Some(nsfw), BUFFER_SIZE).await?;

        cache.add_many(
            image_list
                .images
                .iter()
                .filter_map(|img| img.get_url().ok()),
        );

        Ok(())
    }

    pub async fn get_rand(&self, nsfw: bool) -> Result<Option<Url>, NekosError> {
        let cache = self.get_cache(nsfw);

        if cache.primary_is_empty() {
            let self_clone = self.clone();
            tokio::spawn(async move {
                // TODO: Consider reporting error somehow?
                let _ = self_clone.populate(nsfw).await.is_ok(); // Best effort here, we can always fall back to secondary cache
            });
        }

        if cache.secondary_is_empty() {
            self.populate(nsfw).await?;
        }

        Ok(cache.get_rand().await)
    }
}

impl CacheStatsProvider for NekosClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        let cache = self.get_cache(false);
        let nsfw_cache = self.get_cache(true);

        cache_stats_builder.publish_stat("nekos", "primary_cache", cache.primary_len() as f32);
        cache_stats_builder.publish_stat(
            "nekos",
            "primary_nsfw_cache",
            nsfw_cache.primary_len() as f32,
        );
        cache_stats_builder.publish_stat("nekos", "secondary_cache", cache.secondary_len() as f32);
        cache_stats_builder.publish_stat(
            "nekos",
            "secondary_nsfw_cache",
            nsfw_cache.secondary_len() as f32,
        );
    }
}

impl Default for NekosClient {
    fn default() -> Self {
        Self::new()
    }
}

#[command]
#[bucket("nekos")]
#[description("Get a random neko")]
#[checks(Enabled)]
async fn nekos(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let nsfw = args.single::<NsfwArg>().map(|_| true).unwrap_or(false);

    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let logger = client_data.logger.clone();
    let nekos_client = client_data.nekos_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match nekos_client.get_rand(nsfw).await {
        Ok(Some(url)) => {
            loading.send_ok();
            msg.channel_id.say(&ctx.http, url.as_str()).await?;
        }
        Ok(None) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Both primary and secondary caches are empty. This is not possible.",
                )
                .await?;

            error!(
                logger,
                "Both primary and secondary caches are empty. This is not possible.",
            );
        }
        Err(e) => {
            error!(
                logger,
                "Failed to repopulate nekos cache (secondary cache empty): {}", e
            );

            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to repopulate nekos cache: {}", e),
                )
                .await?;
        }
    }

    Ok(())
}
