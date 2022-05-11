use crate::{
    client_data::{
        CacheStatsBuilder,
        CacheStatsProvider,
    },
    util::{
        ArcAnyhowError,
        DropRemoveFile,
        DropRemovePath,
        EncoderTask,
        RequestMap,
        TimedCache,
        TimedCacheEntry,
    },
    ClientDataKey,
    LoadingReaction,
    TikTokEmbedFlags,
};
use anyhow::{
    ensure,
    Context as _,
};
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
use tokio_stream::StreamExt;
use tracing::{
    error,
    info,
};
use url::Url;

const FILE_SIZE_LIMIT: u64 = 8_000_000;
const ENCODER_PREFERENCE_LIST: &[&str] = &[
    "h264_nvenc",
    "h264_amf",
    "h264_qsv",
    "h264_mf",
    "h264_v4l2m2m",
    "h264_vaapi",
    "h264_omx",
    "libx264",
    "libx264rgb",
];

type VideoDownloadRequestMap = Arc<RequestMap<String, Result<Arc<PathBuf>, ArcAnyhowError>>>;

/// Calculate the target bitrate.
///
/// target_size is in kilobits.
/// target_duration is in seconds.
/// the bitrate is in kilobits
fn calc_target_bitrate(target_size: u64, duration: u64) -> u64 {
    // https://stackoverflow.com/questions/29082422/ffmpeg-video-compression-specific-file-size

    target_size / duration
}

/// TikTok Data
#[derive(Debug, Clone)]
pub struct TikTokData {
    /// The inner client
    client: tiktok::Client,

    /// The encoder task
    encoder_task: EncoderTask,

    /// A cache of post urls => post pages
    pub post_page_cache: TimedCache<String, tiktok::PostPage>,

    /// The path to tiktok's cache dir
    video_download_cache_path: PathBuf,

    /// The request map for making requests for video downloads.
    video_download_request_map: VideoDownloadRequestMap,

    video_encoder: &'static str,
}

impl TikTokData {
    /// Make a new [`TikTokData`].
    pub async fn new(cache_dir: &Path, encoder_task: EncoderTask) -> anyhow::Result<Self> {
        let video_download_cache_path = cache_dir.join("tiktok");

        // TODO: Expand into proper filecache manager
        tokio::fs::create_dir_all(&video_download_cache_path)
            .await
            .context("failed to create tiktok cache dir")?;

        let mut encoders = encoder_task
            .get_encoders(true)
            .await
            .context("failed to get encoders")?;

        // Keep only h264 encoders
        encoders.retain(|encoder| encoder.description.ends_with("(codec h264)"));
        info!("found h264 encoders: {:#?}", encoders);

        let mut best_encoder_index = None;
        for encoder in encoders {
            if let Some(index) = ENCODER_PREFERENCE_LIST
                .iter()
                .position(|name| **name == *encoder.name)
            {
                if best_encoder_index.map_or(true, |best_encoder_index| best_encoder_index > index)
                {
                    best_encoder_index = Some(index);
                }
            }
        }

        let best_encoder_index = best_encoder_index.context("failed to select an encoder")?;
        let best_encoder = ENCODER_PREFERENCE_LIST[best_encoder_index];

        info!("selected encoder '{}'", best_encoder);

        Ok(Self {
            client: tiktok::Client::new(),

            encoder_task,

            post_page_cache: TimedCache::new(),

            video_download_cache_path,
            video_download_request_map: Arc::new(RequestMap::new()),
            video_encoder: best_encoder,
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
        video_duration: u64,
    ) -> anyhow::Result<Arc<PathBuf>> {
        self.video_download_request_map
            .get_or_fetch(id.to_string(), || {
                let client = self.client.client.clone();

                let encoder_task = self.encoder_task.clone();

                let reencoded_file_name = format!("{id}-reencoded.mp4");
                let reencoded_file_path = self.video_download_cache_path.join(&reencoded_file_name);

                let file_name = format!("{id}.{format}");
                let file_path = self.video_download_cache_path.join(&file_name);

                let id = id.to_string();
                let format = format.to_string();
                let url = url.to_string();

                let video_encoder = self.video_encoder;

                async move {
                    match tokio::fs::metadata(&reencoded_file_path).await {
                        Ok(_metadata) => {
                            // The reencoded file is present. Use it.
                            return Ok(Arc::new(reencoded_file_path));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // The transcoded file is not present.
                            // Attempt to use the original file by passing through.
                        }
                        Err(e) => {
                            return Err(e)
                                .context("failed to get metadata of re-encoded file")
                                .map_err(ArcAnyhowError::new);
                        }
                    };

                    // Get the metadata of the raw file.
                    // Download it if needed.
                    let metadata = match tokio::fs::metadata(&file_path).await {
                        Ok(metadata) => {
                            // The reencoded file is present.
                            // Return the metadata to validate its size.
                            metadata
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // File not present. Download it.

                            info!(
                                "downloading tiktok video \
                                with with id `{id}` \
                                from url `{url}` \
                                with format `{format}`"
                            );

                            let result = async {
                                let file_path_tmp =
                                    crate::util::with_push_extension(&file_path, "tmp");
                                let mut file = DropRemoveFile::create(&file_path_tmp)
                                    .await
                                    .context("failed to open file")?;
                                crate::util::download_to_file(&client, &url, &mut file)
                                    .await
                                    .context("failed to download")?;
                                tokio::fs::rename(&file_path_tmp, &file_path)
                                    .await
                                    .context("failed to rename file")?;
                                file.persist();
                                file.metadata().await.context("failed to get file metadata")
                            }
                            .await;

                            result.map_err(ArcAnyhowError::new)?
                        }
                        Err(e) => {
                            return Err(e)
                                .context("failed to get metadata of file")
                                .map_err(ArcAnyhowError::new);
                        }
                    };

                    // If the file is greater than 8mb, we need to reencode it
                    if metadata.len() > FILE_SIZE_LIMIT {
                        let result = async {
                            // We target 7 MB to give ourselves some lee-way.
                            // This merely sets the target bit-rate, and we don't take into account audio size.
                            let target_bitrate = calc_target_bitrate(7_000 * 8, video_duration);
                            let mut reencoded_file_path_tmp = DropRemovePath::new(
                                crate::util::with_push_extension(&reencoded_file_path, "tmp"),
                            );

                            info!(
                                "re-encoding tiktok video `{}` to `{}`\
                                @ video bitrate {}",
                                file_path.display(),
                                reencoded_file_path_tmp.display(),
                                target_bitrate
                            );
                            let mut stream = encoder_task
                                .encode()
                                .input(&file_path)
                                .output(&*reencoded_file_path_tmp)
                                .audio_codec("copy")
                                .video_codec(video_encoder)
                                .video_bitrate(format!("{}K", target_bitrate))
                                .output_format("mp4")
                                .try_send()
                                .await
                                .context("failed to start re-encoding")?;

                            let mut maybe_exit_status = None;
                            while let Some(msg) = stream.next().await {
                                match msg.context("ffmpeg stream error") {
                                    Ok(tokio_ffmpeg_cli::Event::ExitStatus(exit_status)) => {
                                        maybe_exit_status = Some(exit_status);
                                    }
                                    Ok(tokio_ffmpeg_cli::Event::Progress(_progress)) => {
                                        // For now, we don't care about progress as there is no way to report it to the user on discord.
                                    }
                                    Ok(tokio_ffmpeg_cli::Event::Unknown(_)) => {
                                        // We don't care about unkown lines
                                    }
                                    Err(e) => {
                                        error!("{:?}", e);
                                    }
                                }
                            }

                            let exit_status = maybe_exit_status
                                .context("stream did not report an exit status")?;

                            // Validate exit status
                            ensure!(exit_status.success(), "invalid exit status");

                            // Validate file size
                            let metadata = tokio::fs::metadata(&reencoded_file_path_tmp)
                                .await
                                .context("failed to get metadata of encoded file")?;

                            ensure!(
                                metadata.len() < FILE_SIZE_LIMIT,
                                "re-encoded file is larger than {}",
                                FILE_SIZE_LIMIT
                            );

                            // Rename the tmp file to be the actual name.
                            tokio::fs::rename(&*reencoded_file_path_tmp, &reencoded_file_path)
                                .await
                                .context("failed to rename temp file")?;

                            // "Persist" the tmp file, as in don't try to remove it
                            reencoded_file_path_tmp.persist();

                            Ok(())
                        }
                        .await;

                        result.map_err(ArcAnyhowError::new)?;

                        Ok(Arc::new(reencoded_file_path))
                    } else {
                        Ok(Arc::new(file_path))
                    }
                }
            })
            .await
            .map_err(From::from)
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
        let (video_url, video_id, video_format, video_duration) = {
            let post = self.get_post_cached(url.as_str()).await?;
            let post = post.data();
            let item_module_post = post
                .get_item_module_post()
                .context("missing item module post")?;

            let video_url = item_module_post.video.download_addr.clone();
            let video_id = item_module_post.video.id.clone();
            let video_format = item_module_post.video.format.clone();
            let video_duration = item_module_post.video.duration;

            (video_url, video_id, video_format, video_duration)
        };

        let video_path = self
            .get_video_data_cached(
                video_id.as_str(),
                video_format.as_str(),
                video_url.as_str(),
                video_duration,
            )
            .await
            .context("failed to download tiktok video")?;

        msg.channel_id
            .send_message(&ctx.http, |m| m.add_file(&*video_path))
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
