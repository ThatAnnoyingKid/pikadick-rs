use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    ClientDataKey,
};
use anyhow::Context as _;
use crossbeam::queue::ArrayQueue;
use indexmap::set::IndexSet;
use parking_lot::RwLock;
use pikadick_slash_framework::FromOptions;
use rand::Rng;
use serenity::builder::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use std::{
    str::FromStr,
    sync::Arc,
};
use tracing::error;
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

/// A nekos cache
#[derive(Clone, Debug)]
pub struct Cache(Arc<CacheInner>);

impl Cache {
    /// Make a new cache
    pub fn new() -> Self {
        Self(Arc::new(CacheInner {
            primary: ArrayQueue::new(BUFFER_SIZE.into()),
            secondary: RwLock::new(IndexSet::new()),
        }))
    }

    /// Get the size of the primary cache
    pub fn primary_len(&self) -> usize {
        self.0.primary.len()
    }

    /// Get the size of the secondary cache
    pub fn secondary_len(&self) -> usize {
        self.0.secondary.read().len()
    }

    /// Check if the primary cache is emoty
    pub fn primary_is_empty(&self) -> bool {
        self.0.primary.is_empty()
    }

    /// Check if the secondary cache is empty
    pub fn secondary_is_empty(&self) -> bool {
        self.0.secondary.read().is_empty()
    }

    /// Add a url to the cache
    pub fn add(&self, uri: Url) {
        let _ = self.0.primary.push(uri.clone()).is_ok();
        self.0.secondary.write().insert(uri);
    }

    /// Add many urls to the cache
    pub fn add_many<I>(&self, iter: I)
    where
        I: Iterator<Item = Url>,
    {
        let mut secondary = self.0.secondary.write();
        for uri in iter {
            let _ = self.0.primary.push(uri.clone()).is_ok();
            secondary.insert(uri);
        }
    }

    /// Get a random url
    pub async fn get_rand(&self) -> Option<Url> {
        if let Some(uri) = self.0.primary.pop() {
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

/// The inner cache data
#[derive(Debug)]
struct CacheInner {
    primary: ArrayQueue<Url>,
    secondary: RwLock<IndexSet<Url>>,
}

/// The nekos client
#[derive(Clone, Debug)]
pub struct NekosClient {
    client: nekos::Client,

    cache: Cache,
    nsfw_cache: Cache,
}

impl NekosClient {
    /// Make a new nekos client
    pub fn new() -> Self {
        NekosClient {
            client: Default::default(),
            cache: Cache::new(),
            nsfw_cache: Cache::new(),
        }
    }

    /// Get a cache
    fn get_cache(&self, nsfw: bool) -> &Cache {
        if nsfw {
            &self.nsfw_cache
        } else {
            &self.cache
        }
    }

    /// Repopulate a cache
    pub async fn populate(&self, nsfw: bool) -> anyhow::Result<()> {
        let cache = self.get_cache(nsfw);
        let image_list = self
            .client
            .get_random(Some(nsfw), BUFFER_SIZE)
            .await
            .context("failed to get random nekos image list")?;

        cache.add_many(
            image_list
                .images
                .iter()
                .filter_map(|img| img.get_url().ok()),
        );

        Ok(())
    }

    /// Get a random nekos image
    pub async fn get_rand(&self, nsfw: bool) -> anyhow::Result<Url> {
        let cache = self.get_cache(nsfw);

        if cache.primary_is_empty() {
            let self_clone = self.clone();
            tokio::spawn(async move {
                // Best effort here, we can always fall back to secondary cache
                if let Err(e) = self_clone
                    .populate(nsfw)
                    .await
                    .context("failed to get new nekos data")
                {
                    error!("{:?}", e);
                }
            });
        }

        if cache.secondary_is_empty() {
            self.populate(nsfw)
                .await
                .context("failed to populate caches")?;
        }

        cache
            .get_rand()
            .await
            .context("both primary and secondary caches are empty")
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

// TODO:
// Consider adding https://nekos.life/api/v2/endpoints

/// Arguments for the nekos command
#[derive(Debug, Copy, Clone, FromOptions)]
pub struct NekosArguments {
    /// Whether the command should look for nsfw pictures
    pub nsfw: Option<bool>,
}

/// Make a nekos slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("nekos")
        .description("Get a random neko")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("nsfw")
                .kind(pikadick_slash_framework::ArgumentKind::Boolean)
                .description("Whether this should use nsfw results")
                .build()?,
        )
        .on_process(|ctx, interaction, args: NekosArguments| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("failed to get client data");
            let nekos_client = client_data.nekos_client.clone();
            drop(data_lock);

            let content = match nekos_client
                .get_rand(args.nsfw.unwrap_or(false))
                .await
                .context("failed to repopulate nekos caches")
            {
                Ok(url) => url.into(),
                Err(error) => {
                    error!("{error:?}");
                    format!("{error:?}")
                }
            };

            let message_builder = CreateInteractionResponseMessage::new().content(content);
            let response = CreateInteractionResponse::Message(message_builder);

            interaction.create_response(&ctx.http, response).await?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}
