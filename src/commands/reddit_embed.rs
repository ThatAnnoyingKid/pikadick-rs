use crate::{
    checks::{
        ADMIN_CHECK,
        ENABLED_CHECK,
    },
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
use anyhow::{
    bail,
    Context as _,
};
use dashmap::DashMap;
use rand::prelude::IndexedRandom;
use reddit_tube::types::get_video_response::GetVideoResponseOk;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tracing::{
    error,
    info,
    warn,
};
use url::Url;

type SubReddit = String;
type PostId = String;

type LinkVec = Vec<Arc<reddit::Link>>;

// pub struct SubredditPostIdentifier {}

#[derive(Clone)]
pub struct RedditEmbedData {
    reddit_client: reddit::Client,
    reddit_tube_client: reddit_tube::Client,

    pub cache: TimedCache<(SubReddit, PostId), String>,
    pub video_data_cache: TimedCache<String, Box<GetVideoResponseOk>>,
    random_post_cache: Arc<DashMap<String, Arc<(Instant, LinkVec)>>>,
}

impl RedditEmbedData {
    /// Make a new [`RedditEmbedData`].
    pub fn new() -> Self {
        RedditEmbedData {
            reddit_client: reddit::Client::new(),
            reddit_tube_client: reddit_tube::Client::new(),

            cache: Default::default(),
            video_data_cache: TimedCache::new(),
            random_post_cache: Arc::new(DashMap::new()),
        }
    }

    /// Get the original post from a given subreddit and post id.
    ///
    /// This resolves crossposts. Currently only resolves 1 layer.
    pub async fn get_original_post(
        &self,
        subreddit: &str,
        post_id: &str,
    ) -> anyhow::Result<Box<reddit::Link>> {
        let mut post_data = self.reddit_client.get_post(subreddit, post_id).await?;

        if post_data.is_empty() {
            bail!("missing post");
        }

        let mut post_data = post_data
            .swap_remove(0)
            .data
            .into_listing()
            .context("missing post")?
            .children;

        if post_data.is_empty() {
            bail!("missing post");
        }

        let mut post = post_data
            .swap_remove(0)
            .data
            .into_link()
            .context("missing post")?;

        // If cross post, resolve one level. Is it possible to crosspost a crosspost?

        // Remove crosspost list from response...
        let crosspost_parent_list = std::mem::take(&mut post.crosspost_parent_list);
        if let Some(post) = crosspost_parent_list.and_then(|mut l| {
            if l.is_empty() {
                None
            } else {
                Some(l.swap_remove(0))
            }
        }) {
            // TODO: Crossposts are not stored in boxes, but in a vec. We need to unify the return types somehow.
            // Should we choose to move out of a box, or move into a box? Which will be used more?
            Ok(Box::new(post))
        } else {
            Ok(post)
        }
    }

    /// Get video data from reddit.tube.
    ///
    /// Takes a reddit url.
    pub async fn get_video_data(&self, url: &str) -> anyhow::Result<Box<GetVideoResponseOk>> {
        let main_page = self
            .reddit_tube_client
            .get_main_page()
            .await
            .context("failed to get main page")?;
        self.reddit_tube_client
            .get_video(&main_page, url)
            .await
            .context("failed to get video data")?
            .into_result()
            .context("bad video response")
    }

    /// Get video data, but using a cache.
    pub async fn get_video_data_cached(
        &self,
        url: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Box<GetVideoResponseOk>>>> {
        if let Some(response) = self.video_data_cache.get_if_fresh(url) {
            return Ok(response);
        }

        let video_data = self.get_video_data(url).await?;

        Ok(self
            .video_data_cache
            .insert_and_get(url.to_string(), video_data))
    }

    /// Create a video url for a url to a reddit video post.
    pub async fn create_video_url(&self, url: &str) -> anyhow::Result<Url> {
        let maybe_url = self
            .get_video_data_cached(url)
            .await
            .with_context(|| format!("failed to get reddit video info for '{}'", url))
            .map(|video_data| video_data.data().url.clone());

        if let Err(e) = maybe_url.as_ref() {
            warn!("{:?}", e);
        }

        maybe_url
    }

    /// Get a reddit embed url for a given subreddit and post id
    pub async fn get_embed_url(&self, url: &Url) -> anyhow::Result<String> {
        let (subreddit, post_id) = parse_post_url(url).context("failed to parse post")?;

        let original_post = self
            .get_original_post(subreddit, post_id)
            .await
            .context("failed to get reddit post")
            .map_err(|e| {
                warn!("{:?}", e);
                e
            })?;

        if !original_post.is_video {
            return Ok(original_post.url.into());
        }

        self.create_video_url(url.as_str())
            .await
            .map(|url| url.into())
    }

    /// Try to embed a url
    pub async fn try_embed_url(
        &self,
        ctx: &Context,
        msg: &Message,
        url: &Url,
        loading_reaction: &mut Option<LoadingReaction>,
    ) -> anyhow::Result<()> {
        // This is sometimes TOO smart and finds data for invalid urls...
        // TODO: Consider making parsing stricter
        if let Some((subreddit, post_id)) = parse_post_url(url) {
            // Try cache
            let maybe_url = self
                .cache
                .get_if_fresh(&(subreddit.into(), post_id.into()))
                .map(|el| el.data().clone());

            let data = if let Some(value) = maybe_url.clone() {
                Some(value)
            } else {
                self.get_embed_url(url).await.ok()
            };

            if let Some(data) = data {
                self.cache
                    .insert((subreddit.into(), post_id.into()), data.clone());

                // TODO: Consider downloading and reposting?
                msg.channel_id.say(&ctx.http, data).await?;
                if let Some(mut loading_reaction) = loading_reaction.take() {
                    loading_reaction.send_ok();
                }
            }
        } else {
            error!("failed to parse reddit post url");
            // TODO: Maybe expand this to an actual error to give better feedback
        }
        Ok(())
    }

    /// Get a random post url for a subreddit
    pub async fn get_random_post(&self, subreddit: &str) -> anyhow::Result<Option<String>> {
        {
            let urls = self.random_post_cache.get(subreddit);

            if let Some(link) = urls.and_then(|v| {
                let entry = v.value().clone();
                if entry.0.elapsed() > Duration::from_secs(10 * 60) {
                    return None;
                }
                entry.1.choose(&mut rand::thread_rng()).cloned()
            }) {
                let url = self.reddit_link_to_embed_url(&link).await?;
                return Ok(Some(url));
            }
        }

        info!("fetching reddit posts for '{}'", subreddit);
        let mut maybe_url = None;
        let list = self.reddit_client.get_subreddit(subreddit, 100).await?;
        if let Some(listing) = list.data.into_listing() {
            let posts: Vec<Arc<reddit::Link>> = listing
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
                .map(|link| Arc::new(*link))
                .collect();

            let maybe_link = posts.choose(&mut rand::thread_rng()).cloned();
            if let Some(link) = maybe_link {
                maybe_url = Some(self.reddit_link_to_embed_url(&link).await?);
            }

            self.random_post_cache
                .insert(subreddit.to_string(), Arc::new((Instant::now(), posts)));
        }

        Ok(maybe_url)
    }

    /// Convert a reddit link to an embed url
    async fn reddit_link_to_embed_url(&self, link: &reddit::Link) -> anyhow::Result<String> {
        let post_url = format!("https://www.reddit.com{}", link.permalink);

        // Discord should be able to embed non-18 stuff
        if !link.over_18 {
            return Ok(post_url);
        }

        match link.post_hint {
            Some(reddit::PostHint::HostedVideo) => {
                let url = self.create_video_url(&post_url).await?;
                Ok(url.into())
            }
            _ => Ok(link.url.clone().into()),
        }
    }
}

impl CacheStatsProvider for RedditEmbedData {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("reddit_embed", "link_cache", self.cache.len() as f32);
        cache_stats_builder.publish_stat(
            "reddit_embed",
            "video_data_cache",
            self.video_data_cache.len() as f32,
        );
        cache_stats_builder.publish_stat(
            "reddit_embed",
            "random_post_cache",
            self.random_post_cache
                .iter()
                .map(|v| v.value().1.len())
                .sum::<usize>() as f32,
        );
    }
}

impl std::fmt::Debug for RedditEmbedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Replace with manual impl if/when reddit_client becomes debug
        f.debug_struct("RedditEmbedData")
            .field("reddit_tube_client", &self.reddit_tube_client)
            .field("cache", &self.cache)
            .finish()
    }
}

impl Default for RedditEmbedData {
    fn default() -> Self {
        Self::new()
    }
}

// Broken in help:
// #[required_permissions("ADMINISTRATOR")]

#[command("reddit-embed")]
#[description("Enable automaitc reddit embedding for this server")]
#[usage("<enable/disable>")]
#[example("enable")]
#[min_args(1)]
#[max_args(1)]
#[checks(Admin, Enabled)]
async fn reddit_embed(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let db = client_data.db.clone();
    drop(data_lock);

    let enable = match args.trimmed().current().expect("missing arg") {
        "enable" => true,
        "disable" => false,
        arg => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "The argument '{}' is not recognized. Valid: enable, disable",
                        arg
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    // TODO: Probably can unwrap if i add a check to the command
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Missing server id. Are you in a server right now?",
                )
                .await?;
            return Ok(());
        }
    };

    let old_val = db.set_reddit_embed_enabled(guild_id, enable).await?;

    let status_str = if enable { "enabled" } else { "disabled" };

    if enable == old_val {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Reddit embeds are already {} for this server", status_str),
            )
            .await?;
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                format!("Reddit embeds are now {} for this guild", status_str),
            )
            .await?;
    }

    Ok(())
}

/// Gets the subreddit and post id from a reddit url.
///
/// # Returns
/// Returns a tuple or the the subreddit and post id in that order.
pub fn parse_post_url(url: &Url) -> Option<(&str, &str)> {
    // Reddit path:
    // /r/dankmemes/comments/h966lq/davie_is_shookt/

    // Template:
    // /r/<subreddit>/comments/<post_id>/<post_title (irrelevant)>/

    // Parts:
    // r
    // <subreddit>
    // comments
    // <post_id>
    // <post_title>
    // (Nothing, should be empty or not existent)

    let mut iter = url.path_segments()?;

    if iter.next()? != "r" {
        return None;
    }

    let subreddit = iter.next()?;

    if iter.next()? != "comments" {
        return None;
    }

    let post_id = iter.next()?;

    // TODO: Should we reject urls with the wrong ending?

    Some((subreddit, post_id))
}
