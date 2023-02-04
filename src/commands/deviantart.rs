use crate::{
    bot_context::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    BotContext,
    Database,
};
use anyhow::Context as _;
use deviantart::Deviation;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use rand::seq::IteratorRandom;
use std::{
    sync::Arc,
    time::Instant,
};
use tracing::{
    error,
    info,
};
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

const DATA_STORE_NAME: &str = "deviantart";
const COOKIE_KEY: &str = "cookie-store";

/// A caching deviantart client
#[derive(Clone, Debug)]
pub struct DeviantartClient {
    client: deviantart::Client,
    search_cache: TimedCache<String, Vec<Deviation>>,
}

impl DeviantartClient {
    /// Make a new [`DeviantartClient`].
    pub async fn new(db: &Database) -> anyhow::Result<Self> {
        use std::io::BufReader;

        let client = deviantart::Client::new();

        let cookie_data: Option<Vec<u8>> = db
            .store_get(DATA_STORE_NAME, COOKIE_KEY)
            .await
            .context("failed to get cookie data")?;

        match cookie_data {
            Some(cookie_data) => {
                client
                    .load_json_cookies(BufReader::new(std::io::Cursor::new(cookie_data)))
                    .await?;
            }
            None => {
                info!("could not load cookie data");
            }
        }

        Ok(DeviantartClient {
            client,
            search_cache: TimedCache::new(),
        })
    }

    /// Signs in if necessary
    pub async fn sign_in(
        &self,
        db: &Database,
        username: &str,
        password: &str,
    ) -> anyhow::Result<()> {
        if !self.client.is_logged_in_online().await? {
            info!("re-signing in");
            self.client.sign_in(username, password).await?;

            // Store the new cookies
            let cookie_store = self.client.cookie_store.clone();
            let cookie_data = tokio::task::spawn_blocking(move || {
                let mut cookie_data = Vec::with_capacity(1_000_000); // 1 MB
                cookie_store
                    .lock()
                    .expect("cookie store is poisoned")
                    .save_json(&mut cookie_data)
                    .map_err(deviantart::WrapBoxError)?;
                anyhow::Result::<_>::Ok(cookie_data)
            })
            .await??;
            db.store_put(DATA_STORE_NAME, COOKIE_KEY, cookie_data)
                .await?;
        }

        Ok(())
    }

    /// Search for deviantart images with a cache.
    pub async fn search(
        &self,
        db: &Database,
        username: &str,
        password: &str,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Vec<Deviation>>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let start = Instant::now();
        self.sign_in(db, username, password)
            .await
            .context("failed to log in to deviantart")?;
        let mut search_cursor = self.client.search(query, None);
        search_cursor
            .next_page()
            .await
            .context("failed to search")?;
        let list = search_cursor
            .take_current_deviations()
            .expect("missing page")
            .context("failed to process results")?;
        let ret = self.search_cache.insert_and_get(String::from(query), list);

        info!("searched deviantart in {:?}", start.elapsed());

        Ok(ret)
    }
}

impl CacheStatsProvider for DeviantartClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("deviantart", "search_cache", self.search_cache.len());
    }
}

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct DeviantArtOptions {
    #[pikadick_slash_framework(description = "What to look up")]
    query: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("deviantart")
        .description("Get art from deviantart")
        .arguments(DeviantArtOptions::get_argument_params()?.into_iter())
        .on_process(
            |client_data, interaction, args: DeviantArtOptions| async move {
                let deviantart_client = &client_data.inner.deviantart_client;
                let database = &client_data.inner.database;
                let config = &client_data.inner.config;
                let interaction_client = client_data.interaction_client();
                let mut response_data = InteractionResponseDataBuilder::new();

                let query = args.query.as_str();
                info!("Searching for '{query}' on deviantart");

                let result = deviantart_client
                    .search(
                        database,
                        &config.deviantart.username,
                        &config.deviantart.password,
                        query,
                    )
                    .await
                    .with_context(|| format!("failed to search '{query}' on deviantart"));

                match result.as_ref().map(|entry| entry.data()).map(|data| {
                    data.iter()
                        .filter_map(|deviation| {
                            if deviation.is_image() {
                                Some(
                                    deviation
                                        .get_image_download_url()
                                        .or_else(|| deviation.get_fullview_url()),
                                )
                            } else if deviation.is_film() {
                                Some(deviation.get_best_video_url().cloned())
                            } else {
                                None
                            }
                        })
                        .choose(&mut rand::thread_rng())
                }) {
                    Ok(Some(Some(url))) => {
                        response_data = response_data.content(url);
                    }
                    Ok(Some(None)) => {
                        error!("deviantart deviation missing asset url: {:?}", result);
                        response_data =
                            response_data.content("Missing url. This is probably a bug.");
                    }
                    Ok(None) => {
                        response_data = response_data.content("No results");
                    }
                    Err(e) => {
                        error!("{e:?}");
                        response_data = response_data.content(format!("{e:?}"));
                    }
                }

                let response_data = response_data.build();
                let response = InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(response_data),
                };
                interaction_client
                    .create_response(interaction.id, interaction.token.as_str(), &response)
                    .exec()
                    .await
                    .context("failed to send response")?;

                deviantart_client.search_cache.trim();

                Ok(())
            },
        )
        .build()
        .context("failed to build")
}
