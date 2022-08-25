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
use pikadick_slash_framework::ClientData;
use r6stats::UserData;
use std::sync::Arc;
use tracing::{
    error,
    info,
};
use twilight_model::{
    channel::embed::Embed,
    http::interaction::{
        InteractionResponse,
        InteractionResponseType,
    },
};
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
        ImageSource,
    },
    InteractionResponseDataBuilder,
};

#[derive(Clone, Default, Debug)]
pub struct R6StatsClient {
    client: r6stats::Client,
    search_cache: TimedCache<String, UserData>,
}

impl R6StatsClient {
    pub fn new() -> Self {
        Self {
            client: r6stats::Client::new(),
            search_cache: TimedCache::new(),
        }
    }

    /// Get stats
    pub async fn get_stats(
        &self,
        query: &str,
    ) -> Result<Option<Arc<TimedCacheEntry<UserData>>>, r6stats::Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(Some(entry));
        }

        let mut user_list = self.client.search(query).await?;

        if user_list.is_empty() {
            return Ok(None);
        }

        let user = user_list.swap_remove(0);

        self.search_cache.insert(String::from(query), user);

        Ok(self.search_cache.get_if_fresh(query))
    }
}

impl CacheStatsProvider for R6StatsClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("r6stats", "search_cache", self.search_cache.len() as f32);
    }
}

/// Build an embed
fn build_embed(user_data: &UserData) -> anyhow::Result<Embed> {
    let mut embed_builder = EmbedBuilder::new()
        .title(&user_data.username)
        .image(ImageSource::url(user_data.avatar_url_256.as_str())?);

    if let Some(kd) = user_data.kd() {
        embed_builder = embed_builder.field(
            EmbedFieldBuilder::new("Overall Kill / Death", ryu::Buffer::new().format(kd)).inline(),
        );
    }

    if let Some(wl) = user_data.wl() {
        embed_builder = embed_builder.field(
            EmbedFieldBuilder::new("Overall Win / Loss", ryu::Buffer::new().format(wl)).inline(),
        );
    }

    if let Some(stats) = user_data.seasonal_stats.as_ref() {
        embed_builder = embed_builder
            .field(EmbedFieldBuilder::new("MMR", ryu::Buffer::new().format(stats.mmr)).inline())
            .field(
                EmbedFieldBuilder::new("Max MMR", ryu::Buffer::new().format(stats.max_mmr))
                    .inline(),
            )
            .field(
                EmbedFieldBuilder::new("Mean Skill", ryu::Buffer::new().format(stats.skill_mean))
                    .inline(),
            );
    }

    Ok(embed_builder.build())
}

/// Options for r6stats
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct R6StatsOptions {
    /// The user name
    name: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("r6stats")
        .description("Get r6 stats for a user from r6stats")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("name")
                .description("The name of the user")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .required(true)
                .build()?,
        )
        .on_process(
            |client_data, interaction, args: R6StatsOptions| async move {
                let client = client_data.inner.r6stats_client.clone();
                let name = args.name.as_str();
                info!("getting r6 stats for '{name}' using r6stats");

                let result = client
                    .get_stats(name)
                    .await
                    .with_context(|| format!("failed to get stats for '{name}' using r6stats"));

                let interaction_client = client_data.interaction_client();
                let mut response_data = InteractionResponseDataBuilder::new();

                match result.map(|maybe_entry| maybe_entry.map(|entry| build_embed(entry.data()))) {
                    Ok(Some(Ok(embed))) => {
                        response_data = response_data.embeds(std::iter::once(embed));
                    }
                    Ok(None) => {
                        response_data = response_data.content("No results");
                    }
                    Err(e) | Ok(Some(Err(e))) => {
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
                    .create_response(interaction.id, &interaction.token, &response)
                    .exec()
                    .await?;

                client.search_cache.trim();

                Ok(())
            },
        )
        .build()
        .context("failed to build r6stats command")
}
