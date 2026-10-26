use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use bewu_util::AsyncTimedLruCache;
use nd_util::ArcAnyhowError;
use poise::CreateReply;
use serenity::builder::CreateEmbed;
use std::{
    sync::Arc,
    time::Duration,
};
use tracing::error;

const FIVE_MINUTES: Duration = Duration::from_secs(60 * 5);

#[derive(Debug)]
pub struct IqdbClient {
    client: iqdb::Client,
    search_cache: AsyncTimedLruCache<String, Result<Arc<iqdb::SearchResults>, ArcAnyhowError>>,
}

impl IqdbClient {
    pub fn new() -> Self {
        Self {
            client: iqdb::Client::new(),
            search_cache: AsyncTimedLruCache::new(100, FIVE_MINUTES),
        }
    }

    /// Search for an image, with caching
    pub async fn search(&self, query: &str) -> Result<Arc<iqdb::SearchResults>, ArcAnyhowError> {
        self.search_cache
            .get(query.to_string(), || async {
                self.client
                    .search(query)
                    .await
                    .context("Failed to search for image")
                    .map(Arc::new)
                    .map_err(ArcAnyhowError::new)
            })
            .await
    }
}

impl Default for IqdbClient {
    fn default() -> Self {
        Self::new()
    }
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Search IQDB for an image at a url."),
    check = "crate::checks::enabled"
)]
pub async fn iqdb(
    ctx: PoiseContext<'_>,
    #[description = "The url to the image."] url: String,
) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let result = ctx.data().iqdb_client.search(&url).await;
    let mut create_reply = CreateReply::default();
    match result {
        Ok(data) => match data.best_match.as_ref() {
            Some(data) => {
                let create_embed = CreateEmbed::new()
                    .title("IQDB Best Match")
                    .image(data.image_url.as_str())
                    .url(data.url.as_str())
                    .description(data.url.as_str());

                create_reply = create_reply.embed(create_embed);
            }
            None => {
                create_reply = create_reply.content(format!("No results on IQDB for \"{url}\""));
            }
        },
        Err(error) => {
            error!("{error:?}");
            create_reply = create_reply.content(format!("{error:?}"));
        }
    }
    ctx.send(create_reply).await?;

    Ok(())
}
