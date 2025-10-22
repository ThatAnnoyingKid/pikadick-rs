use crate::checks::ENABLED_CHECK;
use rand::prelude::IndexedRandom;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::channel::Message,
};

const FACES: &[&str] = &["(・`ω´・)", ";;w;;", "owo", "UwU", ">w<", "^w^"];

#[command]
#[description("UwUify as phrase")]
#[usage("\"<phrase>\"")]
#[example("\"Hello World!\"")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn uwuify(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let phrase = args.single_quoted::<String>()?;
    msg.channel_id.say(&ctx.http, uwuify_str(&phrase)).await?;
    Ok(())
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

                let face = FACES.choose(&mut rand::thread_rng()).expect("missing face");
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
