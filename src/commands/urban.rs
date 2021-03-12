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
use log::error;
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

/// A Caching Urban Dictionary Client
///
#[derive(Clone, Default, Debug)]
pub struct UrbanClient {
    client: urban_dictionary::Client,
    search_cache: TimedCache<String, urban_dictionary::DefinitionList>,
}

impl UrbanClient {
    /// Make a new [`UrbanClient`].
    ///
    pub fn new() -> UrbanClient {
        Default::default()
    }

    /// Get the top result for a query.
    ///
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
        cache_stats_builder.publish_stat("urban", "search_cache", self.search_cache.len() as f32);
    }
}

#[command]
#[description("Get the top definition from UrbanDictionary.com")]
#[usage("\"<query>\"")]
#[example("\"test\"")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
pub async fn urban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.urban_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);
    let query = args.quoted().trimmed().current().expect("missing arg");

    match client.search(&query).await {
        Ok(entry) => {
            if let Some(entry) = entry.data().list.first() {
                msg.channel_id
                    .send_message(&ctx.http, |m| {
                        m.embed(|e| {
                            e.title(&entry.word)
                                .timestamp(entry.written_on.as_str())
                                .url(&entry.permalink)
                                .field("Definition", entry.get_raw_definition(), false)
                                .field("Example", &entry.get_raw_example(), false)
                                .field("👍", &entry.thumbs_up, true)
                                .field("👎", &entry.thumbs_down, true)
                        })
                    })
                    .await?;

                loading.send_ok();
            } else {
                msg.channel_id.say(&ctx.http, "No results").await?;
            }
        }

        Err(e) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to get urban dictionary search, got: {}", e),
                )
                .await?;
            error!("Failed to get urban dictionary search, got: {}", e);
        }
    }

    client.search_cache.trim();

    Ok(())
}
