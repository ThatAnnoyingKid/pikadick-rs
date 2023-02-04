use crate::{
    commands::{
        deviantart::DeviantartClient,
        iqdb::IqdbClient,
        nekos::NekosClient,
        r6stats::R6StatsClient,
        r6tracker::R6TrackerClient,
        reddit_embed::RedditEmbedData,
        rule34::Rule34Client,
        sauce_nao::SauceNaoClient,
        tiktok_embed::TikTokData,
        urban::UrbanClient,
    },
    config::Config,
    database::Database,
    util::EncoderTask,
};
use anyhow::Context;
use std::{
    collections::BTreeMap,
    sync::Arc,
};
use twilight_http::client::InteractionClient;

/// A tool to build cache stats
#[derive(Debug)]
pub struct CacheStatsBuilder {
    stats: BTreeMap<&'static str, BTreeMap<&'static str, usize>>,
}

impl CacheStatsBuilder {
    /// Make a new [`CacheStatsBuilder`].
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
        }
    }

    /// Publish a stat to a section
    pub fn publish_stat(&mut self, section: &'static str, name: &'static str, value: usize) {
        self.stats.entry(section).or_default().insert(name, value);
    }

    /// Get the inner stats
    pub fn into_inner(self) -> BTreeMap<&'static str, BTreeMap<&'static str, usize>> {
        self.stats
    }
}

impl Default for CacheStatsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A type that can provide cache stats
pub trait CacheStatsProvider {
    /// Publish stats to the provided [`CacheStatsBuilder`].
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder);
}

#[derive(Debug, Clone)]
pub struct BotContext {
    /// The context data
    pub inner: Arc<BotContextInner>,
}

impl BotContext {
    /// Make a new BotContext
    pub async fn new(
        http: twilight_http::Client,
        config: Arc<Config>,
        slash_framework: pikadick_slash_framework::Framework<Self>,
        database: Database,
    ) -> anyhow::Result<Self> {
        let cache_dir = config.cache_dir();

        let nekos_client = NekosClient::new();
        let r6tracker_client = R6TrackerClient::new();
        let r6stats_client = R6StatsClient::new();
        let rule34_client = Rule34Client::new();
        let encoder_task = EncoderTask::new();
        let tiktok_data = TikTokData::new(cache_dir, encoder_task.clone())
            .await
            .context("failed to init tiktok data")?;
        let reddit_embed_data = RedditEmbedData::new();
        let sauce_nao_client = SauceNaoClient::new(config.sauce_nao.api_key.as_str());
        let iqdb_client = IqdbClient::new();
        let xkcd_client = xkcd::Client::new();
        let urban_client = UrbanClient::new();
        let deviantart_client = DeviantartClient::new(&database)
            .await
            .context("failed to init deviantart client")?;
        let insta_client = insta::Client::new();

        Ok(Self {
            inner: Arc::new(BotContextInner {
                http,
                config,
                slash_framework,
                database,

                nekos_client,
                r6tracker_client,
                r6stats_client,
                rule34_client,
                encoder_task,
                tiktok_data,
                reddit_embed_data,
                sauce_nao_client,
                iqdb_client,
                xkcd_client,
                urban_client,
                deviantart_client,
                insta_client,
            }),
        })
    }

    /// Generate cache stats
    ///
    /// Currently, In order for something to show up in cache-stats it must be added here.
    /// More automation is desirable in the future.
    pub fn generate_cache_stats(&self) -> BTreeMap<&'static str, BTreeMap<&'static str, usize>> {
        self.inner.generate_cache_stats()
    }
}

/// The inner bot context.
///
/// The objects here shouldn't implement clone but should internally synchronize if needed.
#[derive(Debug)]
pub struct BotContextInner {
    /// The twilight http client
    pub http: twilight_http::Client,

    /// The bot config
    pub config: Arc<Config>,

    /// The slash_framework
    pub slash_framework: pikadick_slash_framework::Framework<BotContext>,

    /// The database
    pub database: Database,

    /// The nekos client
    pub nekos_client: NekosClient,

    /// R6Tracker client
    pub r6tracker_client: R6TrackerClient,

    /// R6Stats client
    pub r6stats_client: R6StatsClient,

    /// The rule34 client
    pub rule34_client: Rule34Client,

    /// Encoder Task
    pub encoder_task: EncoderTask,

    /// TikTokData
    pub tiktok_data: TikTokData,

    /// The reddit embed data
    pub reddit_embed_data: RedditEmbedData,

    /// The sauce nao client
    pub sauce_nao_client: SauceNaoClient,

    /// The iqdb client
    pub iqdb_client: IqdbClient,

    /// The xkcd client
    pub xkcd_client: xkcd::Client,

    /// The urban dictionary client
    pub urban_client: UrbanClient,

    /// The deviantart client
    pub deviantart_client: DeviantartClient,

    /// The insta client
    pub insta_client: insta::Client,
}

impl BotContextInner {
    /// Generate cache stats
    ///
    /// Currently, In order for something to show up in cache-stats it must be added here.
    /// More automation is desirable in the future.
    fn generate_cache_stats(&self) -> BTreeMap<&'static str, BTreeMap<&'static str, usize>> {
        let mut stat_builder = CacheStatsBuilder::new();

        let cache_stat_providers: &[&dyn CacheStatsProvider] = &[
            // &self.fml_client,
            &self.nekos_client,
            &self.r6stats_client,
            &self.r6tracker_client,
            &self.reddit_embed_data,
            &self.rule34_client,
            // &self.shift_client,
            &self.deviantart_client,
            &self.urban_client,
            &self.iqdb_client,
        ];

        for cache_stat_provider in cache_stat_providers {
            cache_stat_provider.publish_cache_stats(&mut stat_builder);
        }

        stat_builder.into_inner()
    }
}

impl pikadick_slash_framework::ClientData for BotContext {
    /// Get an interaction client
    fn interaction_client(&self) -> InteractionClient<'_> {
        self.inner
            .http
            .interaction(self.inner.config.application_id.into())
    }
}

impl crate::util::twilight_loading_reaction::CloneClient for BotContext {
    fn client(&self) -> &twilight_http::Client {
        &self.inner.http
    }
}
