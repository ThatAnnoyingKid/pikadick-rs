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
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
        ImageSource,
    },
    InteractionResponseDataBuilder,
};

#[derive(Clone, Debug)]
pub struct SauceNaoClient {
    client: sauce_nao::Client,
    search_cache: TimedCache<String, sauce_nao::SearchJson>,
}

impl SauceNaoClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: sauce_nao::Client::new(api_key),
            search_cache: TimedCache::new(),
        }
    }

    /// Search for an image, with caching
    pub async fn search(
        &self,
        query: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<sauce_nao::SearchJson>>> {
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

impl CacheStatsProvider for SauceNaoClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "sauce-nao",
            "search_cache",
            self.search_cache.len() as f32,
        );
    }
}

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct SauceNaoOptions {
    #[pikadick_slash_framework(description = "The image url")]
    url: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("sauce-nao")
        .description("Search SauceNao for an image at a url")
        .check(crate::checks::admin::create_slash_check)
        .arguments(SauceNaoOptions::get_argument_params()?.into_iter())
        .on_process(
            |client_data, interaction, args: SauceNaoOptions| async move {
                let sauce_nao_client = client_data.inner.sauce_nao_client.clone();
                let interaction_client = client_data.interaction_client();
                let img_url = args.url.as_str();
                let mut response_data = InteractionResponseDataBuilder::new();
                let result = sauce_nao_client
                    .search(img_url)
                    .await
                    .context("failed to search for image");

                match result.as_ref().map(|entry| entry.data().results.get(0)) {
                    Ok(Some(data)) => {
                        let mut embed = EmbedBuilder::new()
                            .title("SauceNao Best Match")
                            .image(ImageSource::url(data.header.thumbnail.as_str())?);

                        if let Some(ext_url) = data.data.ext_urls.get(0) {
                            embed = embed.description(ext_url.as_str()).url(ext_url.as_str());
                        }

                        if let Some(source) = data.data.source.as_deref() {
                            embed = embed.field(EmbedFieldBuilder::new("Source", source).inline());
                        }

                        if let Some(eng_name) = data.data.eng_name.as_deref() {
                            embed = embed
                                .field(EmbedFieldBuilder::new("English Name", eng_name).inline());
                        }

                        if let Some(jp_name) = data.data.jp_name.as_deref() {
                            embed = embed
                                .field(EmbedFieldBuilder::new("Japanese Name", jp_name).inline());
                        }

                        response_data = response_data.embeds(std::iter::once(embed.build()));
                    }
                    Ok(None) => {
                        response_data = response_data
                            .content(format!("No results on SauceNao for '{img_url}'"));
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

                Ok(())
            },
        )
        .build()
        .context("failed to build command")
}
