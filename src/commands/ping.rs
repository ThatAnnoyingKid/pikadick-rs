use crate::BotContext;
use anyhow::Context as _;
use pikadick_slash_framework::ClientData;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("ping")
        .description("Respond with pong")
        .on_process(|client_data, interaction, _args: ()| async move {
            let interaction_client = client_data.interaction_client();
            let response_data = InteractionResponseDataBuilder::new()
                .content("pong")
                .build();
            let response = InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response_data),
            };

            interaction_client
                .create_response(interaction.id, &interaction.token, &response)
                .exec()
                .await?;
            Ok(())
        })
        .build()
        .context("failed to build command")
}
