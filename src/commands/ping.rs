use anyhow::Context as _;
use serenity::builder::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("ping")
        .description("Respond with pong")
        .on_process(|ctx, interaction, _args: ()| async move {
            let message_builder = CreateInteractionResponseMessage::new().content("pong");
            let response = CreateInteractionResponse::Message(message_builder);
            interaction.create_response(&ctx.http, response).await?;
            Ok(())
        })
        .build()
        .context("failed to build command")
}
