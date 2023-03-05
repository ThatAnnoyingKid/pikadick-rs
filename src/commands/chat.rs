use crate::ClientDataKey;
use anyhow::{
    ensure,
    Context as _,
};
use tracing::{
    error,
    info,
};

const R6_TRACKER_PROMPT: &str = "When a user asks for rainbox six siege statistics for a person, respond only with \"!r6tracker <playername>\".";

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
            let r6_tracker_client = client_data.r6tracker_client.clone();
            drop(data_lock);

            info!(
                "requesting completion for chat message \"{}\"",
                args.message
            );

            interaction.defer(&ctx.http).await?;

            let chat_result = client
                .chat_completion(
                    "gpt-3.5-turbo",
                    &[
                        open_ai::ChatMessage {
                            // gpt-3.5-turbo currently places low weight on system messages, use a user message.
                            role: "user".into(),
                            content: R6_TRACKER_PROMPT.into(),
                        },
                        open_ai::ChatMessage {
                            role: "user".into(),
                            content: args.message.into(),
                        },
                    ],
                    Some(500),
                )
                .await
                .context("failed to get search results")
                .and_then(|mut response| {
                    ensure!(!response.choices.is_empty(), "missing response choice");
                    Ok(response.choices.swap_remove(0))
                });

            let chat_response = match chat_result {
                Ok(result) => result.message.content,
                Err(error) => {
                    error!("{error:?}");
                    interaction
                        .edit_original_interaction_response(&ctx.http, |res| {
                            res.content(format!("{error:?}"))
                        })
                        .await?;
                    return Ok(());
                }
            };

            match chat_response.split_once(' ') {
                Some((command, rest)) => match command {
                    "!r6tracker" => {
                        let stats = r6_tracker_client
                            .get_stats(rest)
                            .await
                            .context("failed to get r6tracker stats");
                        match stats.as_ref().map(|stats| stats.data()) {
                            Ok(Some(stats)) => {
                                interaction
                                    .edit_original_interaction_response(&ctx.http, |res| {
                                        res.embed(|e| stats.populate_embed(e))
                                    })
                                    .await?;
                            }
                            Ok(None) => {
                                interaction
                                    .edit_original_interaction_response(&ctx.http, |res| {
                                        res.content(format!("User \"{}\" was not found", rest))
                                    })
                                    .await?;
                            }
                            Err(error) => {
                                interaction
                                    .edit_original_interaction_response(&ctx.http, |res| {
                                        res.content(format!("{error:?}"))
                                    })
                                    .await?;
                            }
                        }
                    }
                    _ => {
                        interaction
                            .edit_original_interaction_response(&ctx.http, |res| {
                                res.content(chat_response)
                            })
                            .await?;
                    }
                },
                None => {
                    interaction
                        .edit_original_interaction_response(&ctx.http, |res| {
                            res.content(chat_response)
                        })
                        .await?;
                }
            }

            Ok(())
        })
        .build()
        .context("failed to build chat command")
}
