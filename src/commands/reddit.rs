use crate::BotContext;
use anyhow::Context as _;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use tracing::error;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct RedditOptions {
    subreddit: String,
}

pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("reddit")
        .description("Get a random post from a subreddit")
        .arguments(RedditOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: RedditOptions| async move {
            let reddit_embed_data = client_data.inner.reddit_embed_data.clone();
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();

            let result = reddit_embed_data
                .get_random_post(&args.subreddit)
                .await
                .context("failed fetching posts");

            match result {
                Ok(Some(url)) => {
                    response_data = response_data.content(url);
                }
                Ok(None) => {
                    response_data = response_data.content("No posts found");
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
