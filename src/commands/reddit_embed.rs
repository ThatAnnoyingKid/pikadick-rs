use crate::{
    checks::{
        ADMIN_CHECK,
        ENABLED_CHECK,
    },
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::TimedCache,
    ClientDataKey,
};
use lazy_static::lazy_static;
use reddit::RedditError;
use reddit_tube::{
    types::get_video_response::{
        GetVideoResponseError,
        GetVideoResponseOk,
    },
    GetVideoResponse,
    TubeError,
};
use regex::Regex;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use slog::error;
use std::sync::Arc;
use url::Url;

type SubReddit = String;
type PostId = String;

const DATA_STORE_NAME: &str = "reddit-embed";

lazy_static! {
    /// Source: https://urlregex.com/
    static ref URL_REGEX: Regex = Regex::new(include_str!("./url_regex.txt")).unwrap();
}

#[derive(Clone, Default)]
pub struct RedditEmbedData {
    reddit_client: Arc<reddit::Client>,
    reddit_tube_client: reddit_tube::Client,
    cache: TimedCache<(SubReddit, PostId), String>,
}

impl RedditEmbedData {
    pub fn new() -> Self {
        RedditEmbedData {
            reddit_client: Arc::new(reddit::Client::new()),
            reddit_tube_client: reddit_tube::Client::new(),
            cache: Default::default(),
        }
    }

    /// Gets the subreddit and post id from a reddit url and returns a tuple or the two in that order.
    pub fn parse_post_url<'a>(&self, url: &'a Url) -> Option<(&'a str, &'a str)> {
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

    /// Gets the original post. Resolves crossposts.
    pub async fn get_original_post(
        &self,
        subreddit: &str,
        post_id: &str,
    ) -> Result<Box<reddit::Link>, GetPostError> {
        let mut post_data = self.reddit_client.get_post(subreddit, post_id).await?;

        if post_data.is_empty() {
            return Err(GetPostError::MissingPost);
        }

        let mut post_data = post_data
            .swap_remove(0)
            .data
            .into_listing()
            .ok_or(GetPostError::MissingPost)?
            .children;

        if post_data.is_empty() {
            return Err(GetPostError::MissingPost);
        }

        let mut post = post_data
            .swap_remove(0)
            .data
            .into_link()
            .ok_or(GetPostError::MissingPost)?;

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

    /// Get video data from reddit.tube. Takes a reddit url.
    pub async fn get_video_data(&self, url: &Url) -> Result<GetVideoResponseOk, GetVideoDataError> {
        let main_page = self.reddit_tube_client.get_main_page().await?;
        let video_data = self
            .reddit_tube_client
            .get_video(&main_page, url.as_str())
            .await?;

        match video_data {
            GetVideoResponse::Ok(video_data) => Ok(video_data),
            GetVideoResponse::Error(e) => Err(GetVideoDataError::InvalidResponse(e)),
        }
    }

    /// Process a message and insert an embed if neccesary
    pub async fn process_msg(&self, ctx: &Context, msg: &Message) -> CommandResult {
        let data_lock = ctx.data.read().await;
        let client_data = data_lock.get::<ClientDataKey>().unwrap();
        let logger = client_data.logger.clone();
        let db = client_data.db.clone();
        drop(data_lock);

        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => {
                return Ok(());
            }
        };

        let is_enabled_for_guild = {
            let store = db.get_store(DATA_STORE_NAME).await;
            let key = guild_id.0.to_be_bytes();
            match store.get(key).await {
                Ok(Some(b)) => b,
                Ok(None) => false,
                Err(e) => {
                    error!(
                        logger,
                        "Failed to get reddit-embed guild data for '{}': {}", guild_id, e
                    );
                    false
                }
            }
        };

        if !is_enabled_for_guild || msg.author.bot {
            return Ok(());
        }

        // NOTE: Regex doesn't HAVE to be perfect. Ideally, it just needs to be aggressive since parsing it into a url will weed out invalids.
        let urls: Vec<Url> = URL_REGEX
            .find_iter(&msg.content)
            .filter_map(|url_match| Url::parse(url_match.as_str()).ok())
            .filter(|url| {
                let host_str = match url.host_str() {
                    Some(url) => url,
                    None => return false,
                };

                host_str == "www.reddit.com" || host_str == "reddit.com"
            })
            .collect();

        // Embed for each url
        // NOTE: we short circuit on failure since sending a msg to a channel and failing is most likely a permissions problem,
        // esp. since serenity retries each req once
        for url in urls.iter() {
            // This is sometimes TOO smart and finds data for invalid urls...
            // TODO: Consider making parsing stricter
            if let Some((subreddit, post_id)) = self.parse_post_url(&url) {
                // Try cache
                let maybe_url = self
                    .cache
                    .get_if_fresh(&(subreddit.into(), post_id.into()))
                    .map(|el| el.data().clone());

                let data = if let Some(value) = maybe_url.clone() {
                    Some(value)
                } else {
                    match self.get_original_post(subreddit, post_id).await {
                        Ok(post) => {
                            if !post.is_video {
                                Some(post.url)
                            } else {
                                match self.get_video_data(&url).await {
                                    Ok(video_data) => Some(video_data.url.into_string()),
                                    Err(e) => {
                                        error!(
                                            logger,
                                            "Failed to get reddit video info, got error: {:#?}", e
                                        );
                                        None
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(logger, "Failed to get reddit post, got error: {:#?}", e);
                            None
                        }
                    }
                };

                if let Some(data) = data {
                    self.cache
                        .insert((subreddit.into(), post_id.into()), data.clone());

                    // TODO: Consider downloading and reposting?
                    msg.channel_id.say(&ctx.http, data).await?;
                }
            } else {
                error!(logger, "Failed to parse reddit post url");
                // TODO: Maybe expand this to an actual error to give better feedback
            }
        }

        self.cache.trim();

        Ok(())
    }
}

impl CacheStatsProvider for RedditEmbedData {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat("reddit_embed", "link_cache", self.cache.len() as f32);
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

#[derive(Debug)]
pub enum GetPostError {
    Reddit(RedditError),

    MissingPost,
}

impl From<RedditError> for GetPostError {
    fn from(e: RedditError) -> Self {
        GetPostError::Reddit(e)
    }
}

#[derive(Debug)]
pub enum GetVideoDataError {
    RedditTube(TubeError),
    InvalidResponse(GetVideoResponseError),
}

impl From<TubeError> for GetVideoDataError {
    fn from(e: TubeError) -> GetVideoDataError {
        GetVideoDataError::RedditTube(e)
    }
}

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
    let logger = client_data.logger.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let enable = match args.trimmed().current().unwrap() {
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

    let (old_val, _set_new_data) = {
        let store = db.get_store(DATA_STORE_NAME).await;
        let key = guild_id.0.to_be_bytes();
        let old_val: Option<bool> = match store.get(key).await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    logger,
                    "Failed to get reddit-embed guild data for '{}': {:?}", guild_id, e
                );
                None
            }
        };

        let set_new_data = match store.put(key, enable).await {
            Ok(_) => true,
            Err(e) => {
                error!(
                    logger,
                    "Failed to set reddit-embed guild data for '{}' to '{}': {:?}",
                    guild_id,
                    enable,
                    e
                );
                false
            }
        };

        (old_val.unwrap_or(false), set_new_data)
    };

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
