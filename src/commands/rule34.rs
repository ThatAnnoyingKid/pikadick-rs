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
use log::{
    error,
    info,
};
use rand::seq::SliceRandom;
use rule34::{
    Post,
    RuleError,
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

/// A caching rule34 client
#[derive(Clone, Default, Debug)]
pub struct Rule34Client {
    client: rule34::Client,
    search_cache: TimedCache<String, SearchResult>,
    post_cache: TimedCache<u64, Post>,
}

impl Rule34Client {
    /// Make a new [`Rule34Client`].
    pub fn new() -> Rule34Client {
        Default::default()
    }

    /// Search for a query.
    ///
    /// No normalization is performed on the query, see [`rule34::build_search_query`] for more info.
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Arc<TimedCacheEntry<SearchResult>>, RuleError> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let results = self.client.search(query).await?;
        self.search_cache.insert(String::from(query), results);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("search cache entry expired"))
    }

    /// Get the [`Post`] for a given post id.
    pub async fn get_post(&self, id: u64) -> Result<Arc<TimedCacheEntry<Post>>, RuleError> {
        if let Some(entry) = self.post_cache.get_if_fresh(&id) {
            return Ok(entry);
        }

        let post = self.client.get_post(id).await?;
        self.post_cache.insert(id, post);

        Ok(self
            .post_cache
            .get_if_fresh(&id)
            .expect("post cache entry expired"))
    }
}

impl CacheStatsProvider for Rule34Client {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("rule34", "search_cache", self.search_cache.len() as f32);
        cache_stats_builder.publish_stat("rule34", "post_cache", self.post_cache.len() as f32);
    }
}

#[command]
#[description("Look up rule34 for almost anything")]
#[usage("\"<query>\"")]
#[example("\"test\"")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
async fn rule34(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.rule34_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    let query_str = args.single_quoted::<String>().expect("missing query arg");
    let query = match rule34::build_search_query(query_str.split(|c| c == '_' || c == ' ')) {
        Some(s) => s,
        None => {
            msg.channel_id
                .say(&ctx.http, "Invalid characters in search query")
                .await?;
            return Ok(());
        }
    };

    info!("Searching rule34 for '{}'", query_str);

    match client.search(&query).await {
        Ok(search_results) => {
            let maybe_post_id = search_results
                .data()
                .entries
                .choose(&mut rand::thread_rng())
                .map(|post| post.id);

            if let Some(post_id) = maybe_post_id {
                match client.get_post(post_id).await {
                    Ok(post) => {
                        msg.channel_id
                            .say(&ctx.http, post.data().image_url.as_str())
                            .await?;
                        loading.send_ok();
                    }
                    Err(e) => {
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to get rule34 post, got: {}", e))
                            .await?;
                        error!("Failed to get rule34 post: {}", e);
                    }
                }
            } else {
                msg.channel_id
                    .say(&ctx.http, format!("No results for '{}'", query_str))
                    .await?;
            }
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get rule34 post, got: {}", e))
                .await?;
            error!("Failed to get rule34 search result: {}", e);
        }
    }

    client.search_cache.trim();
    client.post_cache.trim();

    Ok(())
}
