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
use rand::seq::IteratorRandom;
use deviantart::SearchResults;
use log::{
    error,
    info,
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
use url::Url;

/// A caching deviantart client
///
#[derive(Clone, Default, Debug)]
pub struct DeviantartClient {
    client: deviantart::Client,
    search_cache: TimedCache<String, SearchResults>,
    oembed_cache: TimedCache<Url, deviantart::OEmbed>,
}

impl DeviantartClient {
    /// Make a new [`DeviantartClient`].
    ///
    pub fn new() -> Self {
        Default::default()
    }

    /// Search for deviantart images with a cache.
    ///
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Arc<TimedCacheEntry<SearchResults>>, deviantart::Error> {
        if let Some(entry) = self.search_cache.get_if_fresh(query) {
            return Ok(entry);
        }

        let list = self.client.search(query).await?;
        self.search_cache.insert(String::from(query), list);

        Ok(self
            .search_cache
            .get_if_fresh(query)
            .expect("invalid entry"))
    }

    /// Look up a deviantart oembed.
    ///
    pub async fn get_oembed(
        &self,
        url: &Url,
    ) -> Result<Arc<TimedCacheEntry<deviantart::OEmbed>>, deviantart::Error> {
        if let Some(entry) = self.oembed_cache.get_if_fresh(url) {
            return Ok(entry);
        }

        let oembed = self.client.get_oembed(url).await?;
        self.oembed_cache.insert(url.clone(), oembed);

        Ok(self.oembed_cache.get_if_fresh(url).expect("invalid entry"))
    }
}

impl CacheStatsProvider for DeviantartClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "deviantart",
            "search_cache",
            self.search_cache.len() as f32,
        );

        cache_stats_builder.publish_stat(
            "deviantart",
            "oembed_cache",
            self.oembed_cache.len() as f32,
        );
    }
}

#[command]
#[description("Get art from deviantart")]
#[usage("<query>")]
#[example("sun")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
async fn deviantart(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.deviantart_client.clone();
    drop(data_lock);

    let query = args.trimmed().quoted().current().unwrap();

    info!("Searching for '{}' on deviantart", query);

    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.search(&query).await {
        Ok(entry) => {
            let data = entry.data();
            let choice = data.deviations.iter().filter(|d| d.is_image()).choose(&mut rand::thread_rng());

            if let Some(choice) = choice {
                info!("Getting oembed for '{}'", &choice.url);
                match client.get_oembed(&choice.url).await {
                    Ok(oembed) => match oembed.data().thumbnail_url.as_ref() {
                        Some(oembed) => {
                            loading.send_ok();
                            msg.channel_id.say(&ctx.http, &oembed).await?;
                        }
                        None => {
                            msg.channel_id
                                .say(&ctx.http, "Failed to get oembed as it is not an image")
                                .await?;
                        }
                    },
                    Err(e) => {
                        msg.channel_id
                            .say(&ctx.http, format!("Failed to get oembed: {}", e))
                            .await?;
                    }
                }
            } else {
                msg.channel_id.say(&ctx.http, "No Results").await?;
            }
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to search '{}': {}", query, e))
                .await?;

            error!("Failed to search for {} on deviantart: {}", query, e);
        }
    }

    client.search_cache.trim();

    Ok(())
}
