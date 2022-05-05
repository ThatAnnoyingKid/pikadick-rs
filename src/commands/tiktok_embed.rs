use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        ArcAnyhowError,
        RequestMap,
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
    LoadingReaction,
    TikTokEmbedFlags,
};
use anyhow::Context as _;
use serenity::{
    model::prelude::*,
    prelude::*,
};
use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};
use tracing::{
    error,
    info,
    warn,
};
use url::Url;

type VideoDownloadRequestMap =
    Arc<RequestMap<String, Result<Arc<(String, PathBuf)>, ArcAnyhowError>>>;

/// TikTok Data
#[derive(Debug, Clone)]
pub struct TikTokData {
    /// The inner client
    client: tiktok::Client,

    /// A cache of post urls => post pages
    pub post_page_cache: TimedCache<String, tiktok::PostPage>,

    /// The path to tiktok's cache dir
    video_download_cache_path: PathBuf,

    /// The request map for making requests for video downloads.
    video_download_request_map: VideoDownloadRequestMap,
}

impl TikTokData {
    /// Make a new [`TikTokData`].
    pub async fn new(cache_dir: &Path) -> anyhow::Result<Self> {
        let video_download_cache_path = cache_dir.join("tiktok");

        // TODO: Expand into proper filecache manager
        tokio::fs::create_dir_all(&video_download_cache_path)
            .await
            .context("failed to create tiktok cache dir")?;

        Ok(Self {
            client: tiktok::Client::new(),

            post_page_cache: TimedCache::new(),

            video_download_cache_path,
            video_download_request_map: Arc::new(RequestMap::new()),
        })
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
        id: &str,
        format: &str,
        url: &str,
    ) -> anyhow::Result<Arc<PathBuf>> {
        let result = self
            .video_download_request_map
            .get_or_fetch(id.to_string(), || {
                let client = self.client.client.clone();

                let file_name = format!("{id}.{format}");
                let file_path = self.video_download_cache_path.join(&file_name);

                let id = id.to_string();
                let format = format.to_string();
                let url = url.to_string();

                async move {
                    // Open a file.
                    let mut file = match tokio::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&file_path)
                        .await
                    {
                        Ok(file) => file,
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                            return Ok(Arc::new(file_path));
                        }
                        Err(e) => {
                            return Err(e)
                                .context("failed to open file")
                                .map_err(ArcAnyhowError::new);
                        }
                    };

                    // If we didn't early return up above, it is our job to download
                    info!(
                        "downloading tiktok video \
                        with with id `{id}` \
                        from url `{url}` \
                        with format `{format}`"
                    );

                    let result = crate::util::download_to_file(&client, &url, &mut file)
                        .await
                        .context("failed to download");

                    // TODO: Consider downloading to temp file or .part file to avoid these errors
                    if result.is_err() {
                        if let Err(e) =
                            tokio::fs::remove_file(&file_path).await.with_context(|| {
                                format!(
                                    "failed to remove invalid video file `{}` from cache",
                                    file_path.display()
                                )
                            })
                        {
                            error!("{:?}", e);
                        }
                    }

                    result.map_err(ArcAnyhowError::new)?;

                    Ok(Arc::new(file_path))
                }
            })
            .await?;
        Ok(result)
    }

    /// Try embedding a url
    pub async fn try_embed_url(
        &self,
        ctx: &Context,
        msg: &Message,
        url: &Url,
        loading_reaction: &mut Option<LoadingReaction>,
        delete_link: bool,
    ) -> anyhow::Result<()> {
        let (video_url, video_id, video_format) = {
            let post = self.get_post_cached(url.as_str()).await?;
            let post = post.data();
            let item_module_post = post
                .get_item_module_post()
                .context("missing item module post")?;

            let video_url = item_module_post.video.download_addr.clone();
            let video_id = item_module_post.video.id.clone();
            let video_format = item_module_post.video.format.clone();

            (video_url, video_id, video_format)
        };

        let maybe_video_file = self
            .get_video_data_cached(video_id.as_str(), video_format.as_str(), video_url.as_str())
            .await;

        msg.channel_id
            .send_message(&ctx.http, |m| {
                match maybe_video_file.as_deref() {
                    Ok((_, file_path)) => m.add_file(file_path),
                    Err(e) => {
                        // We have the url, lets hope it stays valid
                        warn!("{:?}", e);
                        m.content(video_url.as_str())
                    }
                }
            })
            .await?;

        if let Some(mut loading_reaction) = loading_reaction.take() {
            loading_reaction.send_ok();

            if delete_link {
                msg.delete(&ctx.http)
                    .await
                    .context("failed to delete original message")?;
            }
        }

        Ok(())
    }
}

impl CacheStatsProvider for TikTokData {
    fn publish_cache_stats(&self, cache_stats_builder: &mut CacheStatsBuilder) {
        cache_stats_builder.publish_stat(
            "tiktok_data",
            "post_page_cache",
            self.post_page_cache.len() as f32,
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
