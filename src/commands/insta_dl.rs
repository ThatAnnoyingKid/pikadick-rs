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
async fn insta_dl(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let client = client_data.insta_client.clone();
    drop(data_lock);

    let url = args.trimmed().current().expect("missing url");

    info!("Getting insta download url stats for '{}'", url);
    let mut loading = LoadingReaction::new(ctx.http.clone(), &msg);

    match client.get_post(url).await {
        Ok(post) => {
            if let Some(video_url) = post.video_url.as_ref() {
                loading.send_ok();
                msg.channel_id.say(&ctx.http, video_url).await?;
            } else {
                msg.channel_id
                    .say(&ctx.http, "The url is not a valid video post")
                    .await?;
            }
        }
        Err(e) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to get instagram video url: {}", e),
                )
                .await?;
        }
    }

    Ok(())
}
