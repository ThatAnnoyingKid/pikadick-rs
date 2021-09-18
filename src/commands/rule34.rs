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
use rule34::{
    Post,
    SearchResult,
};
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
    search_cache: TimedCache<String, Vec<SearchResult>>,
    post_cache: TimedCache<u64, Post>,
}

impl Rule34Client {
    /// Make a new [`Rule34Client`].
    pub fn new() -> Rule34Client {
        Rule34Client {
            client: rule34::Client::new(),
            search_cache: TimedCache::new(),
            post_cache: TimedCache::new(),
        }
    }

    /// Search for a query.
    #[tracing::instrument(skip(self))]
    pub async fn search(
        &self,
        tags: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Vec<SearchResult>>>> {
        if let Some(entry) = self.search_cache.get_if_fresh(tags) {
            return Ok(entry);
        }

        let results = self
            .client
            .list()
            .tags(Some(tags))
            .execute()
            .await
            .context("failed to search rule34")?;
        self.search_cache.insert(String::from(tags), results);
        self.search_cache
            .get_if_fresh(tags)
            .context("search cache entry expired")
    }

    /// Get the [`Post`] for a given post id.
    #[tracing::instrument(skip(self))]
    pub async fn get_post(&self, id: u64) -> anyhow::Result<Arc<TimedCacheEntry<Post>>> {
        if let Some(entry) = self.post_cache.get_if_fresh(&id) {
            return Ok(entry);
        }

        let post = self
            .client
            .get_post(id)
            .await
            .context("failed to get post")?;
        self.post_cache.insert(id, post);
        self.post_cache
            .get_if_fresh(&id)
            .context("post cache entry expired")
    }
}

impl CacheStatsProvider for Rule34Client {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("rule34", "search_cache", self.search_cache.len() as f32);
        cache_stats_builder.publish_stat("rule34", "post_cache", self.post_cache.len() as f32);
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

    info!("Searching rule34 for '{}'", query_str);

    match client.search(&query_str).await {
        Ok(search_results) => {
            let maybe_post_id = search_results
                .data()
                .choose(&mut rand::thread_rng())
                .map(|post| post.id);

            if let Some(post_id) = maybe_post_id {
                match client.get_post(post_id).await {
                    Ok(post) => {
                        let post_data = post.data();
                        let image_url = post_data.image_url.as_str();
                        info!("Sending {}", image_url);

                        msg.channel_id.say(&ctx.http, image_url).await?;
                        loading.send_ok();
                    }
                    Err(e) => {
                        error!("{:?}", e);

                        msg.channel_id
                            .say(&ctx.http, format!("Failed to get rule34 post: {:?}", e))
                            .await?;
                    }
                }
            } else {
                info!("No results");

                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("No results for '{}'. Searching is tag based, so make sure to use quotes to seperate tag arguments. ", query_str),
                    )
                    .await?;
            }
        }
        Err(e) => {
            error!("Failed to get search results: {:?}", e);
            msg.channel_id
                .say(&ctx.http, format!("Failed to get rule34 post, got: {}", e))
                .await?;
        }
    }

    client.search_cache.trim();
    client.post_cache.trim();

    Ok(())
}
