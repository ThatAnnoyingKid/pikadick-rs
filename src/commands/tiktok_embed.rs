use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
    LoadingReaction,
    TikTokEmbedFlags,
};
use anyhow::Context as _;
use bytes::Bytes;
use serenity::{
    model::prelude::*,
    prelude::*,
};
use std::sync::Arc;
use url::Url;

/// TikTok Data
#[derive(Debug, Clone)]
pub struct TikTokData {
    /// The inner client
    client: tiktok::Client,

    /// A cache of post urls => post pages
    pub post_page_cache: TimedCache<String, tiktok::PostPage>,

    /// A cache of download urls => video data
    pub video_download_cache: TimedCache<String, Bytes>,
}

impl TikTokData {
    /// Make a new [`TikTokData`].
    pub fn new() -> Self {
        Self {
            client: tiktok::Client::new(),

            post_page_cache: TimedCache::new(),
            video_download_cache: TimedCache::new(),
        }
    }

    /// Get a post page, using the cache if needed
    pub async fn get_post_cached(
        &self,
        url: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<tiktok::PostPage>>> {
        if let Some(post_page) = self.post_page_cache.get_if_fresh(url) {
            return Ok(post_page);
        }

        let post_page = self
            .client
            .get_post(url)
            .await
            .context("failed to get post page")?;

        Ok(self
            .post_page_cache
            .insert_and_get(url.to_string(), post_page))
    }

    /// Get video data, using the cache if needed
    pub async fn get_video_data_cached(
        &self,
        url: &str,
    ) -> anyhow::Result<Arc<TimedCacheEntry<Bytes>>> {
        if let Some(video_data) = self.video_download_cache.get_if_fresh(url) {
            return Ok(video_data);
        }

        let video_data = self
            .client
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(self
            .video_download_cache
            .insert_and_get(url.to_string(), video_data))
    }

    /// Try embedding a url
    pub async fn try_embed_url(
        &self,
        ctx: &Context,
        msg: &Message,
        url: &Url,
        loading_reaction: &mut Option<LoadingReaction>,
    ) -> anyhow::Result<()> {
        let (video_url, _desc) = {
            let post = self.get_post_cached(url.as_str()).await?;
            let post = post.data();
            let item_module_post = post
                .get_item_module_post()
                .context("missing item module post")?;

            let video_url = item_module_post.video.download_addr.clone();
            let desc = item_module_post.desc.clone();

            (video_url, desc)
        };

        let video_data = self.get_video_data_cached(video_url.as_str()).await?;

        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.add_file((&**video_data.data(), "video.mp4"))
            })
            .await?;

        if let Some(mut loading_reaction) = loading_reaction.take() {
            loading_reaction.send_ok();
        }

        Ok(())
    }
}

impl Default for TikTokData {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStatsProvider for TikTokData {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "tiktok_data",
            "post_page_cache",
            self.post_page_cache.len() as f32,
        );

        cache_stats_builder.publish_stat(
            "tiktok_data",
            "video_download_cache",
            self.video_download_cache.len() as f32,
        );
    }
}

/// Options for tiktok-embed
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct TikTokEmbedOptions {
    /// Whether embeds should be enabled for this server
    #[pikadick_slash_framework(description = "Whether embeds should be enabled for this server")]
    enable: Option<bool>,

    /// Whether source messages should be deleted
    #[pikadick_slash_framework(
        rename = "delete-link",
        description = "Whether source messages should be deleted"
    )]
    delete_link: Option<bool>,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    use pikadick_slash_framework::FromOptions;

    pikadick_slash_framework::CommandBuilder::new()
        .name("tiktok-embed")
        .description("Configure tiktok embeds for this server")
        .check(crate::checks::admin::create_slash_check)
        .arguments(TikTokEmbedOptions::get_argument_params()?.into_iter())
        .on_process(|ctx, interaction, args: TikTokEmbedOptions| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock.get::<ClientDataKey>().unwrap();
            let db = client_data.db.clone();
            drop(data_lock);

            let guild_id = match interaction.guild_id {
                Some(id) => id,
                None => {
                    interaction
                        .create_interaction_response(&ctx.http, |res| {
                            res.interaction_response_data(|res| {
                                res.content("Missing server id. Are you in a server right now?")
                            })
                        })
                        .await?;
                    return Ok(());
                }
            };

            let mut set_flags = TikTokEmbedFlags::empty();
            let mut unset_flags = TikTokEmbedFlags::empty();

            if let Some(enable) = args.enable {
                if enable {
                    set_flags.insert(TikTokEmbedFlags::ENABLED);
                } else {
                    unset_flags.insert(TikTokEmbedFlags::ENABLED);
                }
            }

            if let Some(enable) = args.delete_link {
                if enable {
                    set_flags.insert(TikTokEmbedFlags::DELETE_LINK);
                } else {
                    unset_flags.insert(TikTokEmbedFlags::DELETE_LINK);
                }
            }

            let (_old_flags, new_flags) = db
                .set_tiktok_embed_flags(guild_id, set_flags, unset_flags)
                .await?;

            interaction
                .create_interaction_response(&ctx.http, |res| {
                    res.interaction_response_data(|res| {
                        res.embed(|e| {
                            e.title("TikTok Embeds")
                                .field(
                                    "Enabled?",
                                    new_flags.contains(TikTokEmbedFlags::ENABLED),
                                    false,
                                )
                                .field(
                                    "Delete link?",
                                    new_flags.contains(TikTokEmbedFlags::DELETE_LINK),
                                    false,
                                )
                        })
                    })
                })
                .await?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}