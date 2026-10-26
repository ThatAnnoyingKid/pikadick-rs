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

#[derive(Clone, Debug)]
pub struct SauceNaoClient {
    client: sauce_nao::Client,
    search_cache:
        Arc<AsyncTimedLruCache<String, Result<Arc<sauce_nao::OkResponse>, ArcAnyhowError>>>,
}

impl SauceNaoClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: sauce_nao::Client::new(api_key),
            search_cache: AsyncTimedLruCache::new(100, FIVE_MINUTES).into(),
        }
    }

    /// Search for an image, with caching.
    pub async fn search(&self, query: &str) -> Result<Arc<sauce_nao::OkResponse>, ArcAnyhowError> {
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

#[poise::command(
    slash_command,
    description_localized("en-US", "Search SauceNao for an image at a url."),
    check = "crate::checks::enabled"
)]
pub async fn sauce_nao(
    ctx: PoiseContext<'_>,
    #[description = "The url to the image."] url: String,
) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let result = ctx
        .data()
        .sauce_nao_client
        .search(&url)
        .await
        .context("Failed to search for image");

    let mut create_reply = CreateReply::default();
    match result {
        Ok(data) => match data.results.first() {
            Some(data) => {
                let mut create_embed = CreateEmbed::new()
                    .title("SauceNao Best Match")
                    .image(data.header.thumbnail.as_str());
                if let Some(ext_url) = data.data.ext_urls.first() {
                    create_embed = create_embed
                        .description(ext_url.as_str())
                        .url(ext_url.as_str());
                }

                if let Some(source) = data.data.source.as_deref() {
                    create_embed = create_embed.field("Source", source, true);
                }

                if let Some(eng_name) = data.data.eng_name.as_deref() {
                    create_embed = create_embed.field("English Name", eng_name, true);
                }

                if let Some(jp_name) = data.data.jp_name.as_deref() {
                    create_embed = create_embed.field("Jap Name", jp_name, true);
                }

                create_reply = create_reply.embed(create_embed);
            }
            None => {
                create_reply =
                    create_reply.content(format!("No results on SauceNao for \"{url}\""));
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
