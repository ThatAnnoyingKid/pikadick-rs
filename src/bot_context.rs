use crate::{
    commands::{
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
use std::sync::Arc;
use twilight_http::client::InteractionClient;

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
            }),
        })
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
