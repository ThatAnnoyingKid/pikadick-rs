use crate::{
    checks::EnabledCheckData,
    commands::{
        deviantart::DeviantartClient,
        iqdb::IqdbClient,
        reddit_embed::RedditEmbedData,
        sauce_nao::SauceNaoClient,
        tic_tac_toe::TicTacToeData,
        tiktok_embed::TikTokData,
    },
    config::Config,
    database::Database,
    util::EncoderTask,
};
use anyhow::Context;
use serenity::gateway::ShardManager;
use std::{
    fmt::Debug,
    sync::Arc,
};
use tracing::error;

/// The [`ClientData`].
#[derive(Debug)]
pub struct ClientData {
    /// The discord shard_manager
    pub shard_manager: Arc<ShardManager>,

    /// The reddit embed data
    pub reddit_embed_data: RedditEmbedData,
    /// The enabled check data
    pub enabled_check_data: EnabledCheckData,
    /// The insta client data
    pub insta_client: insta::Client,
    /// The deviantart client
    pub deviantart_client: DeviantartClient,
    /// The tic tac toe data
    pub tic_tac_toe_data: TicTacToeData,
    /// The iqdb client
    pub iqdb_client: IqdbClient,
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
        shard_manager: Arc<ShardManager>,
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

            reddit_embed_data: Default::default(),
            enabled_check_data: Default::default(),
            insta_client: insta::Client::new(),
            deviantart_client,
            tic_tac_toe_data: Default::default(),
            iqdb_client: Default::default(),
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
        if let Err(error) = self.encoder_task.shutdown().await {
            error!("{error:?}");
        }
    }
}
