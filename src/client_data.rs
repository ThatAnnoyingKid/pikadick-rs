use crate::{
    checks::EnabledCheckData,
    commands::{
        reddit_embed::RedditEmbedData,
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
    /// The tic tac toe data
    pub tic_tac_toe_data: TicTacToeData,
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
        let cache_dir = config.cache_dir();
        let encoder_task = EncoderTask::new();

        let tiktok_data = TikTokData::new(&cache_dir, encoder_task.clone())
            .await
            .context("failed to init tiktok data")?;

        Ok(ClientData {
            shard_manager,

            reddit_embed_data: Default::default(),
            enabled_check_data: Default::default(),
            insta_client: insta::Client::new(),
            tic_tac_toe_data: Default::default(),
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
