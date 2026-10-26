use crate::{
    Config,
    Database,
    commands::{
        deviantart::DeviantartClient,
        nekos::NekosClient,
        quizizz::QuizizzClient,
        r6tracker::R6TrackerClient,
        rule34::Rule34Client,
        sauce_nao::SauceNaoClient,
        urban::UrbanClient,
    },
};
use anyhow::Context;
use std::sync::Arc;

#[derive(Debug)]
pub struct PoiseData {
    pub deviantart_client: DeviantartClient,
    pub nekos_client: NekosClient,
    pub quizizz_client: QuizizzClient,
    pub r6tracker_client: R6TrackerClient,
    pub rule34_client: Rule34Client,
    pub sauce_nao_client: SauceNaoClient,
    pub urban_client: UrbanClient,
    pub xkcd_client: xkcd::Client,
    pub yodaspeak_client: yodaspeak::Client,
}

impl PoiseData {
    pub async fn new(config: Arc<Config>, database: Database) -> anyhow::Result<Self> {
        let deviantart_client = DeviantartClient::new(&config, database.clone())
            .await
            .context("Failed to create deviantart client")?;
        let rule34_client =
            Rule34Client::new(config.rule34.user_id, &config.rule34.api_key, database);
        let sauce_nao_client = SauceNaoClient::new(&config.sauce_nao.api_key);

        Ok(Self {
            deviantart_client,
            nekos_client: NekosClient::new(),
            quizizz_client: QuizizzClient::new(),
            r6tracker_client: R6TrackerClient::new(),
            rule34_client,
            sauce_nao_client,
            urban_client: UrbanClient::new(),
            xkcd_client: xkcd::Client::new(),
            yodaspeak_client: yodaspeak::Client::new(),
        })
    }
}
