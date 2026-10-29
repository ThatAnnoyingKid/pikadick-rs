use crate::{
    commands::tiktok_embed::TikTokData,
    config::Config,
    database::Database,
    util::EncoderTask,
};
use anyhow::Context;
use std::{
    fmt::Debug,
    sync::Arc,
};
use tracing::error;

/// The [`ClientData`].
#[derive(Debug)]
pub struct ClientData {
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
    pub async fn init(config: Arc<Config>, db: Database) -> anyhow::Result<Self> {
        let cache_dir = config.cache_dir();
        let encoder_task = EncoderTask::new();

        let tiktok_data = TikTokData::new(&cache_dir, encoder_task.clone())
            .await
            .context("Failed to init tiktok data")?;

        Ok(ClientData {
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
