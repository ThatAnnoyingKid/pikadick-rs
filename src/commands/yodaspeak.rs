use crate::ClientDataKey;
use anyhow::Context as _;
use pikadick_slash_framework::FromOptions;
use serenity::builder::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::error;

/// Options for yodaspeak
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct Options {
    #[pikadick_slash_framework(description = "the message to translate")]
    message: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("yodaspeak")
        .description("Translate into what yoda would say.")
        .arguments(Options::get_argument_params()?.into_iter())
        .on_process(|ctx, interaction, args: Options| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock.get::<ClientDataKey>().unwrap();
            let client = client_data.yodaspeak.clone();
            drop(data_lock);

            let result = client
                .translate(args.message.as_str())
                .await
                .context("failed to translate");

            let mut message_builder = CreateInteractionResponseMessage::new();
            match result {
                Ok(translated) => {
                    message_builder = message_builder.content(translated);
                }
                Err(error) => {
                    error!("{error:?}");
                    message_builder = message_builder.content(format!("{error:?}"));
                }
            }

            let response = CreateInteractionResponse::Message(message_builder);
            interaction.create_response(&ctx.http, response).await?;

            Ok(())
        })
        .build()
        .context("failed to build command")
}
