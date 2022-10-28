use crate::BotContext;
use anyhow::Context;
use pikadick_slash_framework::{
    ClientData,
    FromOptions,
};
use rand::prelude::SliceRandom;
use twilight_model::http::interaction::{
    InteractionResponse,
    InteractionResponseType,
};
use twilight_util::builder::InteractionResponseDataBuilder;

const FACES: &[&str] = &["(・`ω´・)", ";;w;;", "owo", "UwU", ">w<", "^w^"];

/// Options for uwuify
#[derive(Debug, pikadick_slash_framework::FromOptions)]
struct UwuifyOptions {
    /// The text to uwuify
    #[pikadick_slash_framework(description = "The text to uwuify")]
    text: String,
}

/// Create a slash command
pub fn create_slash_command() -> anyhow::Result<pikadick_slash_framework::Command<BotContext>> {
    pikadick_slash_framework::CommandBuilder::<BotContext>::new()
        .name("uwuify")
        .description("UwUify a phrase")
        .arguments(UwuifyOptions::get_argument_params()?.into_iter())
        .on_process(|client_data, interaction, args: UwuifyOptions| async move {
            let interaction_client = client_data.interaction_client();
            let mut response_data = InteractionResponseDataBuilder::new();
            response_data = response_data.content(uwuify_str(&args.text));

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
        .context("failed to build uwuify command")
}

/// A rust-optimized version of:
/// ```javascript
/// /// Taken from: https://honk.moe/tools/owo.html
/// var faces = ["(・`ω´・)", ";;w;;", "owo", "UwU", ">w<", "^w^"];
/// function OwoifyText(){
///     v = document.getElementById("textarea").value
///     v = v.replace(/(?:r|l)/g, "w");
///     v = v.replace(/(?:R|L)/g, "W");
///     v = v.replace(/n([aeiou])/g, 'ny$1');
///     v = v.replace(/N([aeiou])/g, 'Ny$1');
///     v = v.replace(/N([AEIOU])/g, 'Ny$1');
///     v = v.replace(/ove/g, "uv");
///     v = v.replace(/\!+/g, " " + faces[Math.floor(Math.random() * faces.length)] + " ");
///     document.getElementById("textarea").value = v
///  };
/// ```
/// This version doesn't use regexes and completes the uwufication in 1 iteration with a lookahead buffer of 2 elements.
/// NOTE: It may be buggy due to its complexity and discrepancies with the js version should be reported on the issue tracker.
pub fn uwuify_str(input: &str) -> String {
    let mut iter = input.chars().peekable();
    let mut buf = None;
    let mut output = String::with_capacity(input.len());

    // Buf has 1 cap so it must be empty in each match arm since we try to fetch a value from it here.
    // We can then treat peek/next as the first value from here on.
    while let Some(c) = buf.take().or_else(|| iter.next()) {
        match c {
            'r' | 'l' => {
                output.push('w');
            }
            'R' | 'L' => {
                output.push('W');
            }
            'n' => {
                if let Some(c) = iter.peek().copied() {
                    if matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') {
                        let c = iter.next().unwrap();

                        output.reserve(3);
                        output.push_str("ny");
                        output.push(c);
                    } else {
                        output.push('n');
                    }
                } else {
                    output.push('n');
                }
            }
            'N' => {
                if let Some(c) = iter.peek().copied().map(|c| c.to_ascii_lowercase()) {
                    if matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') {
                        let c = iter.next().unwrap();

                        output.reserve(3);
                        output.push_str("Ny");
                        output.push(c);
                    } else {
                        output.push('N');
                    }
                } else {
                    output.push('N');
                }
            }
            'o' => {
                if let Some(c) = iter.peek().copied() {
                    if c == 'v' {
                        let _ = iter.next().unwrap();

                        if let Some(c) = iter.peek().copied() {
                            if c == 'e' {
                                let _ = iter.next().unwrap();
                                output.push_str("ove");
                            } else {
                                output.push('o');
                                buf = Some('v');
                                // e is still in iter peek buffer
                            }
                        } else {
                            output.push('o');
                            buf = Some('v');
                        }
                    } else {
                        output.push('o');
                        // v is in iter peek buffer
                    }
                } else {
                    output.push('o');
                }
            }
            '!' => {
                while let Some('!') = iter.next() {
                    // Consume all '!'
                    // TODO: Some variants add a face per '!',
                    // it might make sense to add a feature that does that here
                }

                let face = FACES.choose(&mut rand::thread_rng()).expect("Valid Face");
                output.reserve(face.len() + 2);

                output.push(' ');
                output.push_str(face);
                output.push(' ');
            }
            c => output.push(c),
        }
    }

    output
}
