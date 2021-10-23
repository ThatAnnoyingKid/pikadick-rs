use crate::{
    checks::ENABLED_CHECK,
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::LoadingReaction,
    ClientDataKey,
};
use anyhow::Context as _;
use dashmap::DashMap;
use rand::seq::SliceRandom;
use reddit::PostHint;
use serenity::{
    client::Context,
    framework::standard::{
        macros::*,
        Args,
        CommandResult,
    },
    model::prelude::*,
};
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tracing::info;

/// The Reddit client
#[derive(Clone)]
pub struct RedditClient {
    client: reddit::Client,
    cache: Arc<DashMap<String, (Instant, Vec<String>)>>,
}

impl RedditClient {
    /// Make a new [`RedditClient`].
    pub fn new() -> Self {
        Self {
            client: reddit::Client::new(),
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Get a random post url for a subreddit
    pub async fn get_random_post(&self, subreddit: &str) -> anyhow::Result<Option<String>> {
        {
            let urls = self.cache.get(subreddit);

            if let Some(url) = urls.and_then(|v| {
                let (last_update, urls) = v.value();
                if last_update.elapsed() > Duration::from_secs(10 * 60) {
                    return None;
                }
                urls.choose(&mut rand::thread_rng()).cloned()
            }) {
                return Ok(Some(url));
            }
        }

        info!("fetching reddit posts for '{}'", subreddit);
        let mut maybe_url = None;
        let list = self.client.get_subreddit(subreddit, 100).await?;
        if let Some(listing) = list.data.into_listing() {
            let posts: Vec<_> = listing
                .children
                .into_iter()
                .filter_map(|child| child.data.into_link())
                .filter_map(|post| {
                    if let Some(mut post) = post.crosspost_parent_list {
                        if post.is_empty() {
                            None
                        } else {
                            Some(post.swap_remove(0).into())
                        }
                    } else {
                        Some(post)
                    }
                })
                .filter(|link| {
                    let link_url = link.url.as_str();
                    link.post_hint == Some(PostHint::Image)
                        || link_url.ends_with(".jpg")
                        || link_url.ends_with(".png")
                        || link_url.ends_with(".gif")
                })
                .map(|link| link.url)
                .collect();

            maybe_url = posts.choose(&mut rand::thread_rng()).cloned();

            self.cache
                .insert(subreddit.to_string(), (Instant::now(), posts));
        }

        Ok(maybe_url)
    }
}

impl std::fmt::Debug for RedditClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //  TODO: Use reddit::client's debug if it ever implments it
        f.debug_struct("RedditClient").finish()
    }
}

impl Default for RedditClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStatsProvider for RedditClient {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "reddit",
            "cache",
            self.cache.iter().map(|v| v.value().1.len()).sum::<usize>() as f32,
        );
    }
}

#[command]
#[description("Get a random post from a subreddit")]
#[bucket("default")]
#[min_args(1)]
#[max_args(1)]
#[usage("<subreddit_name>")]
#[example("dogpictures")]
#[checks(Enabled)]
async fn reddit(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.reddit_client.clone();
    drop(data_lock);

    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let subreddit = args.single::<String>().expect("missing arg");
    match client
        .get_random_post(&subreddit)
        .await
        .context("failed fetching posts")
    {
        Ok(Some(url)) => {
            msg.channel_id.say(&ctx.http, url).await?;
            loading.send_ok();
        }
        Ok(None) => {
            msg.channel_id
                .say(&ctx.http, "No image posts found.")
                .await?;
        }
        Err(e) => {
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
        }
    }

    Ok(())
}
