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
use log::info;
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

#[derive(Clone, Debug)]
pub struct InstaClient {
    client: insta::Client,
    cache: TimedCache<String, insta::Post>,
}

impl InstaClient {
    /// Make a new insta client with caching
    pub fn new() -> Self {
        InstaClient {
            client: insta::Client::new(),
            cache: TimedCache::new(),
        }
    }

    /// Get maybe cached post data
    pub async fn get_post(
        &self,
        url: &str,
    ) -> Result<Arc<TimedCacheEntry<insta::Post>>, insta::InstaError> {
        if let Some(entry) = self.cache.get_if_fresh(url) {
            return Ok(entry);
        }

        let post = self.client.get_post(url).await?;
        self.cache.insert(String::from(url), post);

        Ok(self.cache.get_if_fresh(url).expect("Valid insta post data"))
    }
}

impl Default for InstaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStatsProvider for InstaClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("insta-dl", "cache", self.cache.len() as f32);
    }
}

#[command("insta-dl")]
#[description("Get a download url for a instagram video")]
#[usage("<url>")]
#[example("https://www.instagram.com/p/CIlZpXKFfNt/")]
#[checks(Enabled)]
#[min_args(1)]
#[max_args(1)]
async fn insta_dl(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let client = client_data.insta_client.clone();
    drop(data_lock);

    let url = args.trimmed().current().expect("Valid Url");

    info!("Getting insta download url stats for '{}'", url);
    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_post(url).await {
        Ok(post) => {
            if let Some(video_data) = &post.data().video_data {
                loading.send_ok();
                msg.channel_id
                    .say(&ctx.http, video_data.video_url.as_str())
                    .await?;
            } else {
                msg.channel_id
                    .say(&ctx.http, "The url is not a valid video post")
                    .await?;
            }
        }
        Err(e) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to get instagram video url: {}", e),
                )
                .await?;
        }
    }

    client.cache.trim();

    Ok(())
}
