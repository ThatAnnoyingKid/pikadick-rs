use anyhow::Context as _;

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command> {
    pikadick_slash_framework::CommandBuilder::new()
        .name("ping")
        .description("Respond with pong")
        .on_process(|ctx, interaction, _args: ()| async move {
            interaction
                .create_interaction_response(&ctx.http, |res| {
                    res.interaction_response_data(|res| res.content("pong"))
                })
                .await?;
            Ok(())
        })
        .build()
        .context("failed to build command")
}
