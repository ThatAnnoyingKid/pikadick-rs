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
    iqdb::IQDB_COMMAND,
    latency::LATENCY_COMMAND,
    leave::LEAVE_COMMAND,
    quizizz::QUIZIZZ_COMMAND,
    reddit::REDDIT_COMMAND,
    reddit_embed::REDDIT_EMBED_COMMAND,
    sauce_nao::SAUCE_NAO_COMMAND,
    shift::SHIFT_COMMAND,
    stop::STOP_COMMAND,
    system::SYSTEM_COMMAND,
    tic_tac_toe::TIC_TAC_TOE_COMMAND,
    urban::URBAN_COMMAND,
    uwuify::UWUIFY_COMMAND,
    vaporwave::VAPORWAVE_COMMAND,
    xkcd::XKCD_COMMAND,
    zalgo::ZALGO_COMMAND,
};
use anyhow::Context;
use pikadick_slash_framework::FromOptions;

/// Help Options
#[derive(Debug, FromOptions)]
pub struct HelpCommandOptions {
    /// The command
    pub command: Option<String>,
}

/// Create a slash help command
pub fn create_slash_help_command() -> anyhow::Result<pikadick_slash_framework::HelpCommand> {
    pikadick_slash_framework::HelpCommandBuilder::new()
        .description("Get information about commands and their use")
        .argument(
            pikadick_slash_framework::ArgumentParamBuilder::new()
                .name("command")
                .description("The command you need help for")
                .kind(pikadick_slash_framework::ArgumentKind::String)
                .build()?,
        )
        .on_process(
            |ctx, interaction, map, args: HelpCommandOptions| async move {
                interaction
                    .create_interaction_response(&ctx.http, |res| {
                        res.interaction_response_data(|res| {
                            res.embed(|embed| {
                                embed.color(0xF4D665_u32);

                                if let Some(command) = args.command {
                                    let maybe_command = map.get(command.as_str());

                                    match maybe_command {
                                        Some(command) => {
                                            embed
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
                                                embed.field("Arguments", &arguments, false);
                                            }
                                        }
                                        None => {
                                            embed.title("Unknown Command").description(format!(
                                                "Command `{}` was not found.",
                                                command
                                            ));
                                        }
                                    }
                                } else {
                                    embed.title("Help");

                                    let mut description = String::with_capacity(256);
                                    for name in map.keys() {
                                        description.push('`');
                                        description.push_str(name);
                                        description.push('`');
                                        description.push('\n');
                                    }

                                    embed.description(description);
                                }

                                embed
                            })
                        })
                    })
                    .await?;

                Ok(())
            },
        )
        .build()
        .context("failed to build help command")
}
