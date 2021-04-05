use crate::{
    checks::ENABLED_CHECK,
    util::LoadingReaction,
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

    info!("Downloading instagram post '{}'", url);
    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_post(url).await {
        Ok(object) => {
            let file_name = if object.is_video() {
                object.get_video_url_file_name().unwrap_or("video.mp4")
            } else if object.is_image() {
                object.get_image_file_name().unwrap_or("image.png")
            } else {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("The post kind '{}' is unknown", object.kind),
                    )
                    .await?;
                return Ok(());
            };

            let mut buffer = Vec::with_capacity(1_000_000); // 1 MB
            if let Err(e) = client.client.download_object_to(&object, &mut buffer).await {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("The post could not be downloaded: {}", e),
                    )
                    .await?;
                return Ok(());
            };

            msg.channel_id
                .send_files(
                    &ctx.http,
                    std::array::IntoIter::new([(buffer.as_slice(), file_name)]),
                    |m| m,
                )
                .await?;
            loading.send_ok();
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to get instagram post: {}", e))
                .await?;
        }
    }

    Ok(())
}
