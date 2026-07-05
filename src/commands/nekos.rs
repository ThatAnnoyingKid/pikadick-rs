use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedLruCache;
use nd_util::ArcAnyhowError;
use rand::prelude::IndexedRandom;
use std::{
    sync::Arc,
    time::Duration,
};
use tracing::error;
use url::Url;

/// Max images per single request
const NUM_IMAGES: u8 = 100;
const ONE_MINUTE: Duration = Duration::from_secs(60);

/// The nekos client
#[derive(Debug)]
pub struct NekosClient {
    client: nekos::Client,
    cache: AsyncTimedLruCache<Option<bool>, Result<Arc<[Url]>, ArcAnyhowError>>,
}

impl NekosClient {
    /// Make a new nekos client
    pub fn new() -> Self {
        NekosClient {
            client: Default::default(),
            cache: AsyncTimedLruCache::new(3, ONE_MINUTE),
        }
    }

    /// Get a random neko url
    pub async fn get_random(&self, nsfw: Option<bool>) -> anyhow::Result<Url> {
        let urls = self
            .cache
            .get(nsfw, || async {
                self.client
                    .get_random(nsfw, NUM_IMAGES)
                    .await
                    .context("failed to get nekos")
                    .map(|image_list| {
                        image_list
                            .images
                            .iter()
                            .filter_map(|img| img.get_url().ok())
                            .collect()
                    })
                    .map_err(ArcAnyhowError::new)
            })
            .await?;

        let url = urls
            .choose(&mut rand::rng())
            .context("no urls found")?
            .clone();

        Ok(url)
    }
}

impl Default for NekosClient {
    fn default() -> Self {
        Self::new()
    }
}

// TODO:
// Consider adding https://nekos.life/api/v2/endpoints

#[poise::command(
    slash_command,
    description_localized("en-US", "Get a random neko"),
    check = "crate::checks::enabled"
)]
pub async fn nekos(
    ctx: PoiseContext<'_>,
    #[description = "Whether this should use nsfw results. Not specifying includes both."]
    nsfw: Option<bool>,
) -> Result<(), PoiseError> {
    ctx.defer().await?;
    let content = match ctx.data().nekos_client.get_random(nsfw).await {
        Ok(url) => url.into(),
        Err(error) => {
            error!("{error:?}");
            format!("{error:?}")
        }
    };

    ctx.reply(content).await?;

    Ok(())
}
