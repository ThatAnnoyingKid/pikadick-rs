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

/// Options for vaporwave
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct VaporwaveOptions {
    /// The text to vaporwave
    #[pikadick_slash_framework(description = "The text to vaporwave")]
    text: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("vaporwave")
        .description("Vaporwave a phrase")
        .arguments(VaporwaveOptions::get_argument_params()?.into_iter())
        .on_process(
            |client_data, interaction, args: VaporwaveOptions| async move {
                let interaction_client = client_data.interaction_client();
                let mut response_data = InteractionResponseDataBuilder::new();
                response_data = response_data.content(vaporwave_str(&args.text));

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
            },
        )
        .build()
        .context("failed to build vaporwave command")
}

pub fn vaporwave_str(data: &str) -> String {
    data.chars()
        .filter_map(|c| {
            let c = u32::from(c);
            if (33..=270).contains(&c) {
                std::char::from_u32(c + 65248) // unwrap or c ?
            } else {
                Some(char::from(32))
            }
        })
        .collect()
}
