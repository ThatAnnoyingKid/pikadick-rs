use crate::{
    ClientDataKey,
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedCacheCell;
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

#[derive(Debug)]
struct NekosClientInner {
    client: nekos::Client,

    cache: Arc<AsyncTimedCacheCell<Result<Arc<[Url]>, ArcAnyhowError>>>,
    nsfw_cache: Arc<AsyncTimedCacheCell<Result<Arc<[Url]>, ArcAnyhowError>>>,
}

/// The nekos client
#[derive(Clone, Debug)]
pub struct NekosClient {
    inner: Arc<NekosClientInner>,
}

impl NekosClient {
    /// Make a new nekos client
    pub fn new() -> Self {
        NekosClient {
            inner: Arc::new(NekosClientInner {
                client: Default::default(),

                cache: Arc::new(AsyncTimedCacheCell::new(ONE_MINUTE)),
                nsfw_cache: Arc::new(AsyncTimedCacheCell::new(ONE_MINUTE)),
            }),
        }
    }

    /// Get a random neko url
    pub async fn get_random(&self, nsfw: bool) -> anyhow::Result<Url> {
        let cache = if nsfw {
            &self.inner.nsfw_cache
        } else {
            &self.inner.cache
        };

        let urls = cache
            .get(|| async {
                self.inner
                    .client
                    .get_random(Some(nsfw), NUM_IMAGES)
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
    #[description = "Whether this should use nsfw results"] nsfw: Option<bool>,
) -> Result<(), PoiseError> {
    let nsfw = nsfw.unwrap_or(false);

    let data_lock = ctx.serenity_context().data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("failed to get client data");
    let nekos_client = client_data.nekos_client.clone();
    drop(data_lock);

    ctx.defer().await?;
    let content = match nekos_client.get_random(nsfw).await {
        Ok(url) => url.into(),
        Err(error) => {
            error!("{error:?}");
            format!("{error:?}")
        }
    };

    ctx.reply(content).await?;

    Ok(())
}
