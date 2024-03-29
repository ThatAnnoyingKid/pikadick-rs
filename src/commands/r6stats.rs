use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
};
use anyhow::Context as _;
use r6stats::UserData;
use serenity::builder::{
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use std::sync::Arc;
use tracing::{
    error,
    info,
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

/// Options for r6stats
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct R6StatsOptions {
    /// The user name
    name: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
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
        .on_process(|ctx, interaction, args: R6StatsOptions| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("missing client data");
            let client = client_data.r6stats_client.clone();
            drop(data_lock);

            let name = args.name.as_str();

            info!("getting r6 stats for \"{name}\" using r6stats");

            let result = client
                .get_stats(name)
                .await
                .with_context(|| format!("failed to get stats for \"{name}\" using r6stats"));

            let mut message_builder = CreateInteractionResponseMessage::new();
            match result {
                Ok(Some(entry)) => {
                    let data = entry.data();

                    let mut embed_builder = CreateEmbed::new();
                    embed_builder = embed_builder
                        .title(&data.username)
                        .image(data.avatar_url_256.as_str());

                    if let Some(stats) = data.seasonal_stats.as_ref() {
                        embed_builder =
                            embed_builder.field("MMR", ryu::Buffer::new().format(stats.mmr), true);
                        embed_builder = embed_builder.field(
                            "Max MMR",
                            ryu::Buffer::new().format(stats.max_mmr),
                            true,
                        );
                        embed_builder = embed_builder.field(
                            "Mean Skill",
                            ryu::Buffer::new().format(stats.skill_mean),
                            true,
                        );
                    }

                    if let Some(kd) = data.kd() {
                        embed_builder = embed_builder.field(
                            "Overall Kill / Death",
                            ryu::Buffer::new().format(kd),
                            true,
                        );
                    }

                    if let Some(wl) = data.wl() {
                        embed_builder = embed_builder.field(
                            "Overall Win / Loss",
                            ryu::Buffer::new().format(wl),
                            true,
                        );
                    }

                    message_builder = message_builder.embed(embed_builder);
                }
                Ok(None) => message_builder = message_builder.content("No results"),
                Err(error) => {
                    error!("{error:?}");
                    message_builder = message_builder.content(format!("{error:?}"));
                }
            }
            let response_builder = CreateInteractionResponse::Message(message_builder);

            interaction
                .create_response(&ctx.http, response_builder)
                .await?;

            client.search_cache.trim();

            Ok(())
        })
        .build()
        .context("failed to build r6stats command")
}
