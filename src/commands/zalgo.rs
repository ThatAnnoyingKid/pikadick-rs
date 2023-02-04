use crate::BotContext;
use anyhow::Context;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;
use zalgo::ZalgoBuilder;

/// Options for zalgo
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct ZalgoOptions {
    /// The text to zalgoify
    #[pikadick_slash_framework(description = "The text to zalgoify")]
    text: String,

    /// The zalgo max
    #[pikadick_slash_framework(description = "The maximum length of the output")]
    max: Option<usize>,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("zalgo")
        .description("Zalgoify a phrase")
        .arguments(ZalgoOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: ZalgoOptions| async move {
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();

            let input_max = args.max.unwrap_or(2_000);

            let input_len = args.text.chars().count();
            let total = (input_max as f32 - input_len as f32) / input_len as f32;
            let max = (total / 3.0) as usize;

            if max == 0 {
                response_data = response_data
                    .content("The phrase cannot be zalgoified within the given limits");
            } else {
                let output = ZalgoBuilder::new()
                    .set_up(max)
                    .set_down(max)
                    .set_mid(max)
                    .zalgoify(args.text.as_str());
                response_data = response_data.content(output);
            }

            let response_data = response_data.build();
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
        .context("failed to build vaporwave command")
}
