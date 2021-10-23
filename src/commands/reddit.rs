use crate::{
    checks::ENABLED_CHECK,
    util::TimedCache,
    ClientDataKey,
};
use anyhow::Context as _;
use dashmap::DashMap;
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
use std::sync::Arc;

/// The Reddit client
#[derive(Clone)]
pub struct RedditClient {
    client: reddit::Client,
    cache: Arc<DashMap<String, TimedCache<String, String>>>,
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
        let subreddit_cache = self
            .cache
            .entry(subreddit.to_string())
            .or_insert_with(TimedCache::new)
            .value()
            .clone();

        if let Some(url) = subreddit_cache.get_random_if_fresh() {
            return Ok(Some(url.data().clone()));
        }

        let mut maybe_url = None;
        let list = self.client.get_subreddit(subreddit, 100).await?;
        if let Some(listing) = list.data.into_listing() {
            let posts_iter = listing
                .children
                .into_iter()
                .filter_map(|child| child.data.into_link())
                .filter(|link| {
                    link.post_hint == Some(PostHint::Image)
                        || link.url.as_str().ends_with(".jpg")
                        || link.url.as_str().ends_with(".png")
                        || link.url.as_str().ends_with(".gif")
                });

            for post in posts_iter {
                if maybe_url.is_none() {
                    maybe_url = Some(post.url.clone());
                }
                subreddit_cache.insert(post.id, post.url);
            }
        } else {
            return Ok(None);
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

    let subreddit = args.single::<String>().expect("missing arg");
    match client
        .get_random_post(&subreddit)
        .await
        .context("failed fetching posts")
    {
        Ok(Some(url)) => {
            msg.channel_id.say(&ctx.http, url).await?;
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
