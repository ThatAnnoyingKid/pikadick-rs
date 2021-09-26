use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        LoadingReaction,
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
};
use anyhow::Context as _;
use rand::seq::SliceRandom;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
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
    list_cache: TimedCache<String, Vec<rule34::PostListResult>>,
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
    pub async fn list(
        &self,
        tags: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Vec<rule34::PostListResult>>>> {
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

#[command]
#[aliases("r34")]
#[description("Look up rule34 for almost anything")]
#[usage("\"<query>\"")]
#[example("\"test\"")]
#[min_args(1)]
#[checks(Enabled)]
#[bucket("default")]
async fn rule34(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.rule34_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let query_str = rule34::SearchQueryBuilder::new()
        .add_tag_iter(args.raw_quoted())
        .take_query_string();

    info!("searching rule34 for '{}'", query_str);

    match client.list(&query_str).await {
        Ok(list_results) => {
            let maybe_list_result: Option<String> = list_results
                .data()
                .choose(&mut rand::thread_rng())
                .map(|list_result| list_result.file_url.to_string());

            if let Some(file_url) = maybe_list_result {
                info!("sending {}", file_url);
                msg.channel_id.say(&ctx.http, file_url).await?;
                loading.send_ok();
            } else {
                info!("no results");
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("No results for '{}'. Searching is tag based, so make sure to use quotes to seperate tag arguments. ", query_str),
                    )
                    .await?;
            }
        }
        Err(e) => {
            error!("failed to get search results: {:?}", e);
            msg.channel_id
                .say(&ctx.http, format!("Failed to get rule34 post, got: {}", e))
                .await?;
        }
    }

    client.list_cache.trim();

    Ok(())
}
