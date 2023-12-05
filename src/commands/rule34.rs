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
use rand::seq::SliceRandom;
use serenity::builder::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use std::sync::Arc;
use tracing::{
    error,
    info,
};

/// A caching rule34 client
#[derive(Clone, Default, Debug)]
pub struct Rule34Client {
    client: rule34::Client,
    // Ideally, this would be an LRU.
    // However, we would also need to add time tracking to
    // get new data when it goes stale.
    // We would end up duplicating 90% of the logic from [`TimedCache`],
    // so directly using an LRU isn't worth it.
    // However, we could add an LRU based on [`TimedCache`]
    // in the future, or add a setting to it to cap the maximum
    // number of entries.
    list_cache: TimedCache<String, rule34::PostList>,
}

impl Rule34Client {
    /// Make a new [`Rule34Client`].
    pub fn new() -> Rule34Client {
        Rule34Client {
            client: rule34::Client::new(),
            list_cache: TimedCache::new(),
        }
    }

    /// Search for a query.
    #[tracing::instrument(skip(self))]
    pub async fn list(&self, tags: &str) -> anyhow::Result<Arc<TimedCacheEntry<rule34::PostList>>> {
        if let Some(entry) = self.list_cache.get_if_fresh(tags) {
            return Ok(entry);
        }

        let results = self
            .client
            .list_posts()
            .tags(Some(tags))
            .limit(Some(1_000))
            .execute()
            .await
            .context("failed to search rule34")?;
        Ok(self.list_cache.insert_and_get(String::from(tags), results))
    }
}

impl CacheStatsProvider for Rule34Client {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("rule34", "list_cache", self.list_cache.len() as f32);
    }
}

/// Options for the rule34 command
#[derive(Debug, pikadick_slash_framework::FromOptions)]
pub struct Rule34Options {
    // The search query
    query: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("rule34")
        .description("Look up rule34 for almost anything")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("query")
                .description("The search query")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .required(true)
                .build()?,
        )
        .on_process(|ctx, interaction, args: Rule34Options| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("missing client data");
            let client = client_data.rule34_client.clone();
            drop(data_lock);

            let query_str = rule34::SearchQueryBuilder::new()
                .add_tag_iter(args.query.split(' '))
                .take_query_string();

            info!("searching rule34 for \"{query_str}\"");

            let result = client
                .list(&query_str)
                .await
                .context("failed to get search results");

            let mut message_builder = CreateInteractionResponseMessage::new();
            match result {
                Ok(list_results) => {
                    let maybe_list_result: Option<String> = list_results
                        .data()
                        .posts
                        .choose(&mut rand::thread_rng())
                        .map(|list_result| list_result.file_url.to_string());

                    if let Some(file_url) = maybe_list_result {
                        info!("sending \"{file_url}\"");
                        message_builder = message_builder.content(file_url);
                    } else {
                        info!("no results");
                        message_builder =
                            message_builder.content(format!("No results for \"{query_str}\""));
                    }
                }
                Err(error) => {
                    error!("{error:?}");
                    message_builder = message_builder.content(format!("{error:?}"));
                }
            }
            let response = CreateInteractionResponse::Message(message_builder);
            interaction.create_response(&ctx.http, response).await?;

            client.list_cache.trim();

            Ok(())
        })
        .build()
        .context("failed to build rule34 command")
}
