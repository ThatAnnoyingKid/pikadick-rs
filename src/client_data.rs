use crate::{
    checks::EnabledCheckData,
    commands::{
        deviantart::DeviantartClient,
        fml::FmlClient,
        iqdb::IqdbClient,
        nekos::NekosClient,
        quizizz::QuizizzClient,
        r6stats::R6StatsClient,
        r6tracker::R6TrackerClient,
        reddit_embed::RedditEmbedData,
        rule34::Rule34Client,
        sauce_nao::SauceNaoClient,
        shift::ShiftClient,
        tic_tac_toe::TicTacToeData,
        tiktok_embed::TikTokData,
        urban::UrbanClient,
    },
    config::Config,
    database::Database,
    util::EncoderTask,
};
use anyhow::Context;
use serenity::{
    client::bridge::gateway::ShardManager,
    prelude::*,
};
use std::{
    collections::BTreeMap,
    fmt::Debug,
    sync::Arc,
};
use tracing::error;

/// A tool to build cache stats
#[derive(Debug)]
pub struct CacheStatsBuilder {
    stats: BTreeMap<&'static str, BTreeMap<&'static str, f32>>,
}

impl CacheStatsBuilder {
    /// Make a new [`CacheStatsBuilder`].
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
        }
    }

    /// Publish a stat to a section
    pub fn publish_stat(&mut self, section: &'static str, name: &'static str, value: f32) {
        self.stats.entry(section).or_default().insert(name, value);
    }

    /// Get the inner stats
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
    /// Publish stats to the provided [`CacheStatsBuilder`].
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder);
}

/// The [`ClientData`].
#[derive(Debug)]
pub struct ClientData {
    /// The discord shard_manager
    pub shard_manager: Arc<Mutex<ShardManager>>,

    /// The client for nekos
    pub nekos_client: NekosClient,
    /// The R6Stats client
    pub r6stats_client: R6StatsClient,
    /// The r6tracker client
    pub r6tracker_client: R6TrackerClient,
    /// The rule34 client
    pub rule34_client: Rule34Client,
    /// The quizizz client
    pub quizizz_client: QuizizzClient,
    /// The fml client
    pub fml_client: FmlClient,
    /// The shift client
    pub shift_client: ShiftClient,
    /// The reddit embed data
    pub reddit_embed_data: RedditEmbedData,
    /// The enabled check data
    pub enabled_check_data: EnabledCheckData,
    /// The insta client data
    pub insta_client: insta::Client,
    /// The deviantart client
    pub deviantart_client: DeviantartClient,
    /// The urban dictionary client
    pub urban_client: UrbanClient,
    /// The xkcd client
    pub xkcd_client: xkcd::Client,
    /// The tic tac toe data
    pub tic_tac_toe_data: TicTacToeData,
    /// The iqdb client
    pub iqdb_client: IqdbClient,
    /// The sauce nao client
    pub sauce_nao_client: SauceNaoClient,
    /// The open ai client
    pub open_ai_client: open_ai::Client,
    /// TikTokData
    pub tiktok_data: TikTokData,
    /// Encoder Task
    pub encoder_task: EncoderTask,

    /// The database
    pub db: Database,

    /// The config
    pub config: Arc<Config>,
}

impl ClientData {
    /// Init this client data
    pub async fn init(
        shard_manager: Arc<Mutex<ShardManager>>,
        config: Arc<Config>,
        db: Database,
    ) -> anyhow::Result<Self> {
        // TODO: Standardize an async init system with allocated data per command somehow. Maybe boxes?

        let cache_dir = config.cache_dir();
        let encoder_task = EncoderTask::new();

        let deviantart_client = DeviantartClient::new(&db)
            .await
            .context("failed to init deviantart client")?;
        let tiktok_data = TikTokData::new(&cache_dir, encoder_task.clone())
            .await
            .context("failed to init tiktok data")?;

        Ok(ClientData {
            shard_manager,

            nekos_client: Default::default(),
            r6stats_client: Default::default(),
            r6tracker_client: Default::default(),
            rule34_client: Default::default(),
            quizizz_client: Default::default(),
            fml_client: FmlClient::new(config.fml.key.to_string()),
            shift_client: ShiftClient::new(),
            reddit_embed_data: Default::default(),
            enabled_check_data: Default::default(),
            insta_client: insta::Client::new(),
            deviantart_client,
            urban_client: Default::default(),
            xkcd_client: Default::default(),
            tic_tac_toe_data: Default::default(),
            iqdb_client: Default::default(),
            sauce_nao_client: SauceNaoClient::new(config.sauce_nao.api_key.as_str()),
            open_ai_client: open_ai::Client::new(config.open_ai.api_key.as_str()),
            tiktok_data,
            encoder_task,

            db,

            config,
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
            &self.deviantart_client,
            &self.urban_client,
            &self.iqdb_client,
        ];

        for cache_stat_provider in cache_stat_providers {
            cache_stat_provider.publish_cache_stats(&mut stat_builder);
        }

        stat_builder.into_inner()
    }

    /// Shutdown anything that needs to be shut down.
    ///
    /// Errors are logged to the console,
    /// but not returned to the user as it is assumed that they don't matter in the middle of a shutdown.
    pub async fn shutdown(&self) {
        if let Err(e) = self.encoder_task.shutdown().await {
            error!("{:?}", e);
        }
    }
}
