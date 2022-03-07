use crate::{
    checks::{
        ADMIN_CHECK,
        ENABLED_CHECK,
    },
    ClientDataKey,
    LoadingReaction,
};
use anyhow::Context as _;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::Message,
    prelude::*,
};
use url::Url;

/// TikTok Data
#[derive(Debug, Clone)]
pub struct TikTokData {
    client: tiktok::Client,
}

impl TikTokData {
    /// Make a new [`TikTokData`].
    pub fn new() -> Self {
        Self {
            client: tiktok::Client::new(),
        }
    }

    /// Try embedding a url
    pub async fn try_embed_url(
        &self,
        ctx: &Context,
        msg: &Message,
        url: &Url,
        loading_reaction: &mut Option<LoadingReaction>,
    ) -> anyhow::Result<()> {
        let post_page = self
            .client
            .get_post(url.as_str())
            .await
            .context("failed to get post page")?;

        let video_url = post_page
            .get_video_download_url()
            .context("missing video url")?;

        let video_data = self
            .client
            .client
            .get(video_url.as_str())
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        msg.channel_id
            .send_message(&ctx.http, |m| m.add_file((&*video_data, "video.mp4")))
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

#[command("tiktok-embed")]
#[description("Enable automaitc tiktok embedding for this server")]
#[usage("<enable/disable>")]
#[example("enable")]
#[min_args(1)]
#[max_args(1)]
#[checks(Admin, Enabled)]
async fn tiktok_embed(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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

    let old_val = db.set_tiktok_embed_enabled(guild_id, enable).await?;

    let status_str = if enable { "enabled" } else { "disabled" };

    if enable == old_val {
        msg.channel_id
            .say(
                &ctx.http,
                format!("TikTok embeds are already {} for this server", status_str),
            )
            .await?;
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                format!("TikTok embeds are now {} for this guild", status_str),
            )
            .await?;
    }

    Ok(())
}
