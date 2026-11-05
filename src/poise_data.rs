use crate::{
    Config,
    Database,
    commands::{
        deviantart::DeviantartClient,
        iqdb::IqdbClient,
        nekos::NekosClient,
        quizizz::QuizizzClient,
        r6tracker::R6TrackerClient,
        rule34::Rule34Client,
        sauce_nao::SauceNaoClient,
        tic_tac_toe::TicTacToeData,
        tiktok_embed::TikTokData,
        urban::UrbanClient,
    },
    util::EncoderTask,
};
use anyhow::Context;
use std::sync::Arc;
use tracing::error;

#[derive(Debug)]
pub struct PoiseData {
    pub config: Arc<Config>,
    pub database: Database,
    pub deviantart_client: DeviantartClient,
    pub encoder_task: EncoderTask,
    pub iqdb_client: IqdbClient,
    pub nekos_client: NekosClient,
    pub quizizz_client: QuizizzClient,
    pub r6tracker_client: R6TrackerClient,
    pub rule34_client: Rule34Client,
    pub sauce_nao_client: SauceNaoClient,
    pub tic_tac_toe_data: TicTacToeData,
    pub tiktok_data: TikTokData,
    pub urban_client: UrbanClient,
    pub xkcd_client: xkcd::Client,
    pub yodaspeak_client: yodaspeak::Client,
}

impl PoiseData {
    pub async fn new(config: Arc<Config>, database: Database) -> anyhow::Result<Self> {
        let cache_dir = config.cache_dir();

        let deviantart_client = DeviantartClient::new(&config, database.clone())
            .await
            .context("Failed to create deviantart client")?;
        let encoder_task = EncoderTask::new();
        let rule34_client = Rule34Client::new(
            config.rule34.user_id,
            &config.rule34.api_key,
            database.clone(),
        );
        let sauce_nao_client = SauceNaoClient::new(&config.sauce_nao.api_key);
        let tic_tac_toe_data = TicTacToeData::new(database.clone());
        let tiktok_data = TikTokData::new(&cache_dir, encoder_task.clone())
            .await
            .context("Failed to init tiktok data")?;

        Ok(Self {
            config,
            database,
            deviantart_client,
            encoder_task,
            iqdb_client: IqdbClient::new(),
            nekos_client: NekosClient::new(),
            quizizz_client: QuizizzClient::new(),
            r6tracker_client: R6TrackerClient::new(),
            rule34_client,
            sauce_nao_client,
            tic_tac_toe_data,
            tiktok_data,
            urban_client: UrbanClient::new(),
            xkcd_client: xkcd::Client::new(),
            yodaspeak_client: yodaspeak::Client::new(),
        })
    }

    /// Shutdown anything that needs to be shut down.
    ///
    /// Errors are logged to the console,
    /// but not returned to the user as it is assumed that they don't matter in the middle of a shutdown.
    pub async fn shutdown(&self) {
        if let Err(error) = self
            .encoder_task
            .shutdown()
            .await
            .context("Failed to shutdown encoder task")
        {
            error!("{error:?}");
        }
    }
}
