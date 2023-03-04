use crate::ClientDataKey;
use anyhow::{
    ensure,
    Context as _,
};
use tracing::{
    error,
    info,
};

/// Options
#[derive(Debug, pikadick_slash_framework::FromOptions)]
pub struct Options {
    // The message
    message: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("chat")
        .description("Chat with pikadick")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("message")
                .description("The message")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .required(true)
                .build()?,
        )
        .on_process(|ctx, interaction, args: Options| async move {
            let data_lock = ctx.data.read().await;
            let client_data = data_lock
                .get::<ClientDataKey>()
                .expect("missing client data");
            let client = client_data.open_ai_client.clone();
            drop(data_lock);

            info!(
                "requesting completion for chat message \"{}\"",
                args.message
            );

            let result = client
                .chat_completion(
                    "gpt-3.5-turbo",
                    &[open_ai::ChatMessage {
                        role: "user".into(),
                        content: args.message.into(),
                    }],
                    Some(500),
                )
                .await
                .context("failed to get search results")
                .and_then(|mut response| {
                    ensure!(!response.choices.is_empty(), "missing response choice");
                    Ok(response.choices.swap_remove(0))
                });

            interaction
                .create_interaction_response(&ctx.http, |res| {
                    res.interaction_response_data(|res| match result {
                        Ok(result) => res.content(result.message.content),
                        Err(error) => {
                            error!("{error:?}");
                            res.content(format!("{error:?}"))
                        }
                    })
                })
                .await?;

            Ok(())
        })
        .build()
        .context("failed to build chat command")
}
