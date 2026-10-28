use crate::{
    Config,
    Database,
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedLruCache;
use deviantart::Deviation;
use nd_util::ArcAnyhowError;
use rand::seq::IteratorRandom;
use std::{
    io::BufReader,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tracing::{
    error,
    info,
    warn,
};

const DATA_STORE_NAME: &str = "deviantart";
const COOKIE_KEY: &str = "cookie-store";

const FIVE_MINUTES: Duration = Duration::from_secs(60 * 5);

/// A caching deviantart client
#[derive(Debug)]
pub struct DeviantartClient {
    username: String,
    password: String,
    client: deviantart::Client,
    database: Database,
    search_cache: AsyncTimedLruCache<String, Result<Arc<[Deviation]>, ArcAnyhowError>>,
}

impl DeviantartClient {
    /// Make a new [`DeviantartClient`].
    pub async fn new(config: &Config, database: Database) -> anyhow::Result<Self> {
        let client = deviantart::Client::new();

        let cookie_data: Option<Vec<u8>> = database
            .store_get(DATA_STORE_NAME, COOKIE_KEY)
            .await
            .context("Failed to get cookie data")?;

        match cookie_data {
            Some(cookie_data) => {
                // Ignore cookies if they fail to parse.
                if let Err(error) = client
                    .load_json_cookies(BufReader::new(std::io::Cursor::new(cookie_data)))
                    .await
                    .context("Failed to load JSON cookies")
                {
                    warn!("{error:?}");
                }
            }
            None => {
                info!("Could not load cookie data");
            }
        }

        let search_cache = AsyncTimedLruCache::new(100, FIVE_MINUTES);

        Ok(DeviantartClient {
            username: config.deviantart.username.clone(),
            password: config.deviantart.password.clone(),
            client,
            database,
            search_cache,
        })
    }

    /// Signs in if necessary
    pub async fn sign_in(&self) -> anyhow::Result<()> {
        if !self.client.is_logged_in_online().await? {
            info!("Re-signing in");
            self.client.login(&self.username, &self.password).await?;

            // Store the new cookies
            let cookie_store = self.client.cookie_store.clone();
            let cookie_data = tokio::task::spawn_blocking(move || {
                let mut cookie_data = Vec::with_capacity(1_000_000); // 1 MB
                cookie_store
                    .lock()
                    .expect("Cookie store is poisoned")
                    .save_json(&mut cookie_data)
                    .map_err(deviantart::WrapBoxError)?;
                anyhow::Ok(cookie_data)
            })
            .await??;
            self.database
                .store_put(DATA_STORE_NAME, COOKIE_KEY, &cookie_data)
                .await?;
        }

        Ok(())
    }

    /// Search for deviantart images with a cache.
    pub async fn search(&self, query: &str) -> Result<Arc<[Deviation]>, ArcAnyhowError> {
        self.search_cache
            .get(query.to_string(), || async {
                let start = Instant::now();
                let result = async {
                    self.sign_in()
                        .await
                        .context("Failed to log in to deviantart")?;

                    let mut search_cursor = self.client.search(query, None);
                    search_cursor
                        .next_page()
                        .await
                        .context("Failed to search")?;

                    let list = search_cursor
                        .take_current_deviations()
                        .expect("Missing page")
                        .context("Failed to process results")?;

                    anyhow::Ok(list)
                }
                .await
                .map(Arc::from)
                .map_err(ArcAnyhowError::new);

                let elapsed = start.elapsed();
                info!("Searched deviantart in {elapsed:?}");

                result
            })
            .await
    }
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Search for art on deviantart")
)]
pub async fn deviantart(
    ctx: PoiseContext<'_>,
    #[description = "The deviantart search query."] query: String,
) -> Result<(), PoiseError> {
    ctx.defer().await?;

    info!("Searching for \"{query}\" on deviantart");
    let result = ctx.data().deviantart_client.search(&query).await;
    match result {
        Ok(data) => {
            let choice = data
                .iter()
                .filter_map(|deviation| {
                    if deviation.is_image() {
                        Some(
                            deviation
                                .get_image_download_url()
                                .or_else(|| deviation.get_fullview_url(Default::default()).ok()),
                        )
                    } else if deviation.is_film() {
                        Some(deviation.get_best_video_url().cloned())
                    } else {
                        None
                    }
                })
                .choose(&mut rand::rng());

            if let Some(choice) = choice {
                if let Some(url) = choice {
                    ctx.reply(url).await?;
                } else {
                    ctx.reply("Missing url. This is probably a bug.").await?;
                    error!("DeviantArt deviation missing asset url");
                }
            } else {
                ctx.reply("No Results").await?;
            }
        }
        Err(error) => {
            ctx.reply(format!("Failed to search \"{query}\": {error:?}"))
                .await?;

            error!("Failed to search for \"{query}\" on deviantart: {error:?}");
        }
    }

    Ok(())
}
