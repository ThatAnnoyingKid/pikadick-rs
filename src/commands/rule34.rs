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
use rule34::{
    Post,
    RuleError,
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

#[derive(Clone, Default, Debug)]
pub struct Rule34Client {
    client: rule34::Client,
    search_cache: TimedCache<String, Post>,
}

impl Rule34Client {
    pub fn new() -> Rule34Client {
        Default::default()
    }

    /// Get the top result for a query. No normalization is performed on the query, see `rule34::build_search_query` for more info.
    pub async fn get_entry(
        &self,
        query: &str,
    ) -> Result<Option<Arc<TimedCacheEntry<Post>>>, RuleError> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(Some(entry));
        }

        let results = self.client.search(query).await?;
        let entries = &results.entries;
        if entries.is_empty() {
            return Ok(None);
        }

        let entry = match entries.first().as_ref().and_then(|p| p.as_ref()) {
            Some(p) => p,
            None => {
                return Ok(None);
            }
        };

        let data = self.client.get_post(entry.id).await?;

        self.search_cache.insert(String::from(query), data);

        Ok(self.search_cache.get_if_fresh(query))
    }

    pub fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("rule34", "search_cache", self.search_cache.len() as f32);
    }
}

impl CacheStatsProvider for Rule34Client {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("rule34", "search_cache", self.search_cache.len() as f32);
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
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.rule34_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    let query = match args
        .single_quoted::<String>()
        .map(|s| rule34::build_search_query(s.split(|c| c == '_' || c == ' ')))?
    {
        Some(s) => s,
        None => {
            msg.channel_id
                .say(&ctx.http, "Invalid chars in search query")
                .await?;
            return Ok(());
        }
    };

    match client.get_entry(&query).await {
        Ok(Some(entry)) => {
            msg.channel_id
                .say(&ctx.http, entry.data().image_url.as_str())
                .await?;

            loading.send_ok();
        }
        Ok(None) => {
            msg.channel_id.say(&ctx.http, "No results").await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get rule34 post, got: {}", e))
                .await?;
        }
    }

    client.search_cache.trim();

    Ok(())
}
