use crate::{
    checks::EnabledCheckData,
    commands::{
        deviantart::DeviantartClient,
        fml::FmlClient,
        insta_dl::InstaClient,
        nekos::NekosClient,
        polldaddy::PollDaddyClient,
        quizizz::QuizizzClient,
        r6stats::R6StatsClient,
        r6tracker::R6TrackerClient,
        reddit_embed::RedditEmbedData,
        rule34::Rule34Client,
        shift::ShiftClient,
    },
    config::Config,
    database::Database,
};
use serenity::{
    client::bridge::gateway::ShardManager,
    prelude::*,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::Debug,
    sync::Arc,
};

#[derive(Debug)]
pub struct CacheStatsBuilder {
    stats: BTreeMap<&'static str, BTreeMap<&'static str, f32>>,
}

impl CacheStatsBuilder {
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
        }
    }

    pub fn publish_stat(&mut self, section: &'static str, name: &'static str, value: f32) {
        self.stats.entry(section).or_default().insert(name, value);
    }

    pub fn into_inner(self) -> BTreeMap<&'static str, BTreeMap<&'static str, f32>> {
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
    /// Publish stats to the provided CacheStatsBuilder
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder);
}

#[derive(Debug)]
pub struct ClientData {
    pub shard_manager: Arc<Mutex<ShardManager>>,

    pub nekos_client: NekosClient,
    pub r6stats_client: R6StatsClient,
    pub r6tracker_client: R6TrackerClient,
    pub rule34_client: Rule34Client,
    pub polldaddy_client: PollDaddyClient,
    pub quizizz_client: QuizizzClient,
    pub fml_client: FmlClient,
    pub shift_client: ShiftClient,
    pub reddit_embed_data: RedditEmbedData,
    pub enabled_check_data: EnabledCheckData,
    pub insta_client: InstaClient,
    pub deviantart_client: DeviantartClient,

    pub db: Database,

    pub config: Arc<Config>,
}

impl ClientData {
    pub async fn init(
        shard_manager: Arc<Mutex<ShardManager>>,
        config: Config,
        db: Database,
    ) -> Result<Self, Box<dyn Error>> {
        // TODO: Standardize an async init system with allocated data per command somehow. Maybe boxes?

        Ok(ClientData {
            shard_manager,

            nekos_client: Default::default(),
            r6stats_client: Default::default(),
            r6tracker_client: Default::default(),
            rule34_client: Default::default(),
            polldaddy_client: Default::default(),
            quizizz_client: Default::default(),
            fml_client: FmlClient::new(config.fml().key().to_string()),
            shift_client: ShiftClient::new(),
            reddit_embed_data: Default::default(),
            enabled_check_data: Default::default(),
            insta_client: Default::default(),
            deviantart_client: Default::default(),

            db,

            config: Arc::new(config),
        })
    }

    /// Generate cache stats
    /// Currently, In order for something to show up in cache-stats it must be added here.
    /// More automation is desirable in the future.
    pub fn generate_cache_stats(&self) -> BTreeMap<&'static str, BTreeMap<&'static str, f32>> {
        let mut stat_builder = CacheStatsBuilder::new();

        let cache_stat_providers: &[&dyn CacheStatsProvider] = &[
            &self.fml_client,
            &self.nekos_client,
            &self.r6stats_client,
            &self.r6tracker_client,
            &self.reddit_embed_data,
            &self.rule34_client,
            &self.shift_client,
            &self.insta_client,
            &self.deviantart_client,
        ];

        for cache_stat_provider in cache_stat_providers {
            cache_stat_provider.publish_cache_stats(&mut stat_builder);
        }

        stat_builder.into_inner()
    }
}
