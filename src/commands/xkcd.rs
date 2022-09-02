use crate::BotContext;
use anyhow::Context;
use pikadick_slash_framework::ClientData;
use tracing::error;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("xkcd")
        .description("Get a random comic from Xkcd")
        .on_process(|client_data, interaction, _args: ()| async move {
            let xkcd_client = client_data.inner.xkcd_client.clone();
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();

            let result = xkcd_client
                .get_random()
                .await
                .context("failed to get xkcd comic");

            match result {
                Ok(data) => {
                    response_data = response_data.content(data);
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
