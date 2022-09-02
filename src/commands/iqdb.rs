use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    BotContext,
};
use anyhow::Context as _;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use std::sync::Arc;
use tracing::error;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::embed::{
    EmbedBuilder,
    ImageSource,
};

#[derive(Clone, Debug)]
pub struct IqdbClient {
    client: iqdb::Client,
    search_cache: TimedCache<String, iqdb::SearchResults>,
}

impl IqdbClient {
    pub fn new() -> Self {
        Self {
            client: iqdb::Client::new(),
            search_cache: TimedCache::new(),
        }
    }

    /// Search for an image, with caching
    pub async fn search(
        &self,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<iqdb::SearchResults>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let search_results = self
            .client
            .search(query)
            .await
            .context("failed to search for image")?;

        self.search_cache
            .insert(String::from(query), search_results);

        self.search_cache
            .get_if_fresh(query)
            .context("cache data expired")
    }
}

impl Default for IqdbClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStatsProvider for IqdbClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("iqdb", "search_cache", self.search_cache.len() as f32);
    }
}

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct IqdbOptions {
    #[pikadick_slash_framework(description = "The url to look up")]
    url: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("iqdb")
        .description("Search IQDB for an image at a url")
        .arguments(IqdbOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: IqdbOptions| async move {
            let iqdb_client = client_data.inner.iqdb_client.clone();
            let interaction_client = client_data.interaction_client();

            interaction_client
                .create_response(
                    interaction.id,
                    interaction.token.as_str(),
                    &InteractionResponse {
                        kind: InteractionResponseType::DeferredChannelMessageWithSource,
                        data: None,
                    },
                )
                .exec()
                .await
                .context("failed to send response")?;
            let update_response = interaction_client.update_response(interaction.token.as_str());

            let img_url = args.url.as_str();
            let result = iqdb_client
                .search(img_url)
                .await
                .context("failed to search for image");

            let update_response = match result
                .as_ref()
                .map(|entry| entry.data().best_match.as_ref())
            {
                Ok(Some(data)) => {
                    let embed = EmbedBuilder::new()
                        .title("IQDB Best Match")
                        .image(ImageSource::url(data.image_url.as_str())?)
                        .url(data.url.as_str())
                        .description(data.url.as_str());

                    update_response.embeds(Some(&[embed.build()]))?.exec()
                }
                Ok(None) => update_response
                    .content(Some(&format!("No results on iqdb for '{img_url}'")))?
                    .exec(),
                Err(e) => {
                    error!("{e:?}");
                    update_response.content(Some(&format!("{e:?}")))?.exec()
                }
            };

            update_response.await.context("failed to update response")?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}
