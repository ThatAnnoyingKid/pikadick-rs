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
};
use anyhow::Context as _;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use tracing::error;
use twilight_model::{
    http::interaction::{
        InteractionResponse,
        InteractionResponseType,
    },
    util::datetime::Timestamp,
};
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
    },
    InteractionResponseDataBuilder,
};

/// A Caching Urban Dictionary Client
#[derive(Clone, Default, Debug)]
pub struct UrbanClient {
    client: urban_dictionary::Client,
    search_cache: TimedCache<String, urban_dictionary::DefinitionList>,
}

impl UrbanClient {
    /// Make a new [`UrbanClient`].
    pub fn new() -> UrbanClient {
        Default::default()
    }

    /// Get the top result for a query.
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Arc<TimedCacheEntry<urban_dictionary::DefinitionList>>, urban_dictionary::Error>
    {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let results = self.client.lookup(query).await?;
        self.search_cache.insert(String::from(query), results);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("recently aquired entry expired"))
    }
}

impl CacheStatsProvider for UrbanClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("urban", "search_cache", self.search_cache.len());
    }
}

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct UrbanOptions {
    #[pikadick_slash_framework(description = "The word or words to look up")]
    query: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("urban")
        .description("Get the top definition from UrbanDictionary.com")
        .arguments(UrbanOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: UrbanOptions| async move {
            let urban_client = &client_data.inner.urban_client;
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();

            let result = urban_client
                .search(args.query.as_str())
                .await
                .context("failed to search urban dictionary");

            match result.as_ref().map(|entry| entry.data().list.first()) {
                Ok(Some(entry)) => {
                    let mut thumbs_up_buf = itoa::Buffer::new();
                    let mut thumbs_down_buf = itoa::Buffer::new();

                    let written_on =
                        time::OffsetDateTime::parse(entry.written_on.as_str(), &Rfc3339)
                            .map(|datetime| datetime.unix_timestamp_nanos() / 1_000)
                            .context("invalid timestamp")
                            .and_then(|nanos| {
                                i64::try_from(nanos).context("timestamp out of range")
                            })
                            .and_then(|micros| {
                                Timestamp::from_micros(micros).context("failed to create timestamp")
                            })?;

                    let embed = EmbedBuilder::new()
                        .title(&entry.word)
                        .timestamp(written_on)
                        .url(entry.permalink.as_str())
                        .field(EmbedFieldBuilder::new(
                            "Definition",
                            &entry.get_raw_definition(),
                        ))
                        .field(EmbedFieldBuilder::new("Example", &entry.get_raw_example()))
                        .field(
                            EmbedFieldBuilder::new("👍", thumbs_up_buf.format(entry.thumbs_up))
                                .inline(),
                        )
                        .field(
                            EmbedFieldBuilder::new("👎", thumbs_down_buf.format(entry.thumbs_down))
                                .inline(),
                        )
                        .build();

                    response_data = response_data.embeds([embed]);
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

            urban_client.search_cache.trim();

            Ok(())
        })
        .build()
        .context("failed to build command")
}
