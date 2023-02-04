use crate::{
    util::get_extension_from_url,
    BotContext,
};
use anyhow::{
    bail,
    Context as _,
};
use bytes::Bytes;
use insta::MediaType;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use tracing::{
    error,
    info,
};
use twilight_model::http::{
    attachment::Attachment,
    interaction::{
        InteractionResponse,
        InteractionResponseType,
    },
};
use twilight_util::builder::InteractionResponseDataBuilder;

// TODO: Cache results
/// Download an instagram post
async fn download_post(client: &insta::Client, url: &str) -> anyhow::Result<(Bytes, String)> {
    let post_page = client
        .get_post_page(url)
        .await
        .context("failed to get instagram post page")?;
    let media_info = client
        .get_media_info(post_page.media_id)
        .await
        .context("failed to get item")?;
    let media_item = media_info.items.first().context("missing media item")?;

    let url = match media_item.media_type {
        MediaType::Photo => {
            let image_versions2_candidate = media_item
                .get_best_image_versions2_candidate()
                .context("failed to select an image_versions2_candidate")?;
            &image_versions2_candidate.url
        }
        MediaType::Video => {
            let video_version = media_item
                .get_best_video_version()
                .context("failed to get the best video version")?;

            &video_version.url
        }
        media_type => {
            bail!("unsupported media type `{media_type:?}`");
        }
    };

    let extension = get_extension_from_url(url).context("missing image extension")?;
    let file_name = format!("{}.{}", media_item.code, extension);

    let data = client
        .client
        .get(url.as_str())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok((data, file_name))
}

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct InstaOptions {
    #[pikadick_slash_framework(description = "The instagram url")]
    url: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("insta-dl")
        .description("Download an instagram video or photo")
        .arguments(InstaOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: InstaOptions| async move {
            let insta_client = client_data.inner.insta_client.clone();
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();

            let url = args.url.as_str();
            info!("downloading instagram post '{url}'");

            let result = download_post(&insta_client, url)
                .await
                .context("failed to download post");

            match result {
                Ok((post_data, file_name)) => {
                    let attachment = Attachment::from_bytes(file_name, post_data.to_vec(), 0);
                    response_data = response_data.attachments([attachment]);
                }
                Err(e) => {
                    error!("{e:?}");
                    response_data = response_data.content(format!("{e:?}"));
                }
            }

            let response_data = response_data.build();
            let response = InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response_data),
            };
            interaction_client
                .create_response(interaction.id, interaction.token.as_str(), &response)
                .exec()
                .await
                .context("failed to send response")?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}
