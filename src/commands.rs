pub mod cache_stats;
pub mod cmd;
pub mod deviantart;
pub mod fml;
pub mod insta_dl;
pub mod invite;
pub mod iqdb;
pub mod latency;
pub mod leave;
pub mod nekos;
pub mod ping;
pub mod quizizz;
pub mod r6stats;
pub mod r6tracker;
pub mod reddit;
pub mod reddit_embed;
pub mod rule34;
pub mod sauce_nao;
pub mod shift;
pub mod stop;
pub mod system;
pub mod tic_tac_toe;
pub mod tiktok_embed;
pub mod urban;
pub mod uwuify;
pub mod vaporwave;
pub mod xkcd;
pub mod zalgo;

pub use crate::commands::{
    cache_stats::CACHE_STATS_COMMAND,
    cmd::CMD_COMMAND,
    deviantart::DEVIANTART_COMMAND,
    fml::FML_COMMAND,
    insta_dl::INSTA_DL_COMMAND,
    invite::INVITE_COMMAND,
    latency::LATENCY_COMMAND,
    leave::LEAVE_COMMAND,
    quizizz::QUIZIZZ_COMMAND,
    reddit_embed::REDDIT_EMBED_COMMAND,
    shift::SHIFT_COMMAND,
    stop::STOP_COMMAND,
    system::SYSTEM_COMMAND,
    tic_tac_toe::TIC_TAC_TOE_COMMAND,
    uwuify::UWUIFY_COMMAND,
    vaporwave::VAPORWAVE_COMMAND,
    zalgo::ZALGO_COMMAND,
};
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
use twilight_util::builder::{
    embed::{
        EmbedBuilder,
        EmbedFieldBuilder,
    },
    InteractionResponseDataBuilder,
};

/// Help Options
#[derive(Debug, FromOptions)]
pub struct HelpCommandOptions {
    /// The command
    pub command: Option<String>,
}

/// Create a slash help command
pub fn create_slash_help_command(
) -> anyhow::Result<pikadick_slash_framework::HelpCommand<BotContext>> {
    pikadick_slash_framework::HelpCommandBuilder::<BotContext>::new()
        .description("Get information about commands and their use")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("command")
                .description("The command you need help for")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .build()?,
        )
        .on_process(
            |client_data, interaction, map, args: HelpCommandOptions| async move {
                let interaction_client = client_data.interaction_client();
                let mut embed = EmbedBuilder::new().color(0xF4D665_u32);
                if let Some(command) = args.command {
                    let maybe_command = map.get(command.as_str());
                    match maybe_command {
                        Some(command) => {
                            embed = embed
                                .title(command.name())
                                .description(command.description());

                            if !command.arguments().is_empty() {
                                let mut arguments = String::with_capacity(256);
                                for argument in command.arguments().iter() {
                                    arguments.push_str("**");
                                    arguments.push_str(argument.name());
                                    arguments.push_str("**");

                                    arguments.push_str(": ");
                                    arguments.push_str(argument.description());
                                }
                                embed =
                                    embed.field(EmbedFieldBuilder::new("Arguments", &arguments));
                            }
                        }
                        None => {
                            embed = embed
                                .title("Unknown Command")
                                .description(format!("Command `{command}` was not found."));
                        }
                    }
                } else {
                    embed = embed.title("Help");

                    let mut description = String::with_capacity(256);
                    for name in map.keys() {
                        description.push('`');
                        description.push_str(name);
                        description.push('`');
                        description.push('\n');
                    }

                    embed = embed.description(description);
                }

                let response_data = InteractionResponseDataBuilder::new()
                    .embeds(std::iter::once(embed.build()))
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
            },
        )
        .build()
        .context("failed to build help command")
}
