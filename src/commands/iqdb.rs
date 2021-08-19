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
use tracing::error;

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

#[command]
#[description("Search IQDB for an image at a url")]
#[usage("<img_url>")]
#[example("https://konachan.com/image/5982d8946ae503351e960f097f84cd90/Konachan.com%20-%20330136%20animal%20nobody%20original%20signed%20yutaka_kana.jpg")]
#[checks(Enabled)]
#[min_args(1)]
#[max_args(1)]
#[bucket("default")]
async fn iqdb(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.iqdb_client.clone();
    drop(data_lock);

    let query = args.trimmed().current().expect("missing query");

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    match client
        .search(query)
        .await
        .context("failed to search for image")
    {
        Ok(data) => {
            let data = data.data();
            match data.best_match.as_ref() {
                Some(data) => {
                    msg.channel_id
                        .send_message(&ctx.http, |m| {
                            m.embed(|e| {
                                e.title("IQDB Best Match")
                                    .image(data.image_url.as_str())
                                    .url(data.url.as_str())
                                    .description(data.url.as_str())
                            })
                        })
                        .await?;

                    loading.send_ok();
                }
                None => {
                    msg.channel_id
                        .say(&ctx.http, format!("No results on iqdb for '{}'", query))
                        .await?;
                }
            }
        }
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
            error!("{:?}", e);
        }
    }

    Ok(())
}
