use crate::{
    checks::EnabledCheckData,
    commands::{
        deviantart::DeviantartClient,
        fml::FmlClient,
        quizizz::QuizizzClient,
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
    fmt::Debug,
    sync::Arc,
};
use tracing::error;

/// The [`ClientData`].
#[derive(Debug)]
pub struct ClientData {
    /// The discord shard_manager
    pub shard_manager: Arc<Mutex<ShardManager>>,

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
    /// The sauce nao client
    pub sauce_nao_client: SauceNaoClient,
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
            sauce_nao_client: SauceNaoClient::new(config.sauce_nao.api_key.as_str()),
            tiktok_data,
            encoder_task,

            db,

            config,
        })
    }

    /// Shutdown anything that needs to be shut down.
    ///
    /// Errors are logged to the console,
    /// but not returned to the user as it is assumed that they don't matter in the middle of a shutdown.
    pub async fn shutdown(&self) {
        if let Err(e) = self.encoder_task.shutdown().await {
            error!("{e:?}");
        }
    }
}
