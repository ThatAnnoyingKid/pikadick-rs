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
    builder::{
        CreateEmbed,
        CreateMessage,
    },
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::{
        prelude::*,
        timestamp::Timestamp,
    },
    prelude::*,
};
use std::sync::Arc;
use tracing::error;

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
            .expect("recently acquired entry expired"))
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
#[bucket("default")]
pub async fn urban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.urban_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);
    let query = args.quoted().trimmed().current().expect("missing arg");

    match client
        .search(query)
        .await
        .context("failed to search urban dictionary")
    {
        Ok(entry) => {
            if let Some(entry) = entry.data().list.first() {
                let mut thumbs_down_buf = itoa::Buffer::new();

                let mut embed_builder = CreateEmbed::new()
                    .title(&entry.word)
                    .url(entry.permalink.as_str())
                    .field("Definition", entry.get_raw_definition(), false)
                    .field("Example", entry.get_raw_example(), false)
                    .field("👍", entry.thumbs_up.to_string(), true)
                    .field("👎", thumbs_down_buf.format(entry.thumbs_down), true);

                match Timestamp::parse(entry.written_on.as_str())
                    .context("failed to parse timestamp")
                {
                    Ok(timestamp) => {
                        embed_builder = embed_builder.timestamp(timestamp);
                    }
                    Err(error) => {
                        error!("{error}");
                    }
                }

                let message_builder = CreateMessage::new().embed(embed_builder);

                msg.channel_id
                    .send_message(&ctx.http, message_builder)
                    .await?;

                loading.send_ok();
            } else {
                msg.channel_id.say(&ctx.http, "No results").await?;
            }
        }

        Err(error) => {
            error!("{error:?}");
            msg.channel_id.say(&ctx.http, format!("{error:?}")).await?;
        }
    }

    client.search_cache.trim();

    Ok(())
}
