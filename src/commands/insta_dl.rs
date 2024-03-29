use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
    ClientDataKey,
};
use anyhow::{
    bail,
    Context as _,
};
use bytes::Bytes;
use insta::MediaType;
use serenity::{
    builder::{
        CreateAttachment,
        CreateMessage,
    },
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use tracing::info;
use url::Url;

#[command("insta-dl")]
#[description("Download an instagram video or photo")]
#[usage("<url>")]
#[example("https://www.instagram.com/p/CIlZpXKFfNt/")]
#[checks(Enabled)]
#[min_args(1)]
#[max_args(1)]
#[bucket("insta-dl")]
async fn insta_dl(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.insta_client.clone();
    drop(data_lock);

    let url = args.trimmed().current().expect("missing url");

    info!("downloading instagram post \"{url}\"");
    let mut loading = LoadingReaction::new(ctx.http.clone(), msg);

    let result = async {
        let post = client
            .get_post(url)
            .await
            .context("failed to get instagram post")?;
        download_post(&client.client, &post)
            .await
            .context("failed to download post")
    }
    .await;

    match result {
        Ok((post_data, file_name)) => {
            let file_builder = CreateAttachment::bytes(post_data, file_name);
            let message_builder = CreateMessage::new().add_file(file_builder);

            msg.channel_id
                .send_message(&ctx.http, message_builder)
                .await?;
            loading.send_ok();
        }
        Err(error) => {
            msg.channel_id.say(&ctx.http, format!("{error:?}")).await?;
        }
    }

    Ok(())
}

// TODO: Cache results
/// Download an instagram post
async fn download_post<'a>(
    client: &reqwest::Client,
    post_page: &'a insta::AdditionalDataLoaded,
) -> anyhow::Result<(Bytes, String)> {
    let post_page_item = post_page.items.first().context("missing post item")?;

    let url = match post_page_item.media_type {
        MediaType::Photo => {
            let image_versions2_candidate = post_page_item
                .get_best_image_versions2_candidate()
                .context("failed to select an image_versions2_candidate")?;
            &image_versions2_candidate.url
        }
        MediaType::Video => {
            let video_version = post_page_item
                .get_best_video_version()
                .context("failed to get the best video version")?;

            &video_version.url
        }
        media_type => {
            bail!("Unsupported media type `{media_type:?}`",);
        }
    };

    let code = &post_page_item.code;
    let extension = get_extension_from_url(url).context("missing image extension")?;
    let file_name = format!("{code}.{extension}");

    let data = client
        .get(url.as_str())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok((data, file_name))
}

/// Get the file extension from a url
fn get_extension_from_url(url: &Url) -> Option<&str> {
    Some(url.path_segments()?.next_back()?.rsplit_once('.')?.1)
}
