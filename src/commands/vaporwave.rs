use crate::checks::ENABLED_CHECK;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::channel::Message,
};

#[command]
#[description("Vaporwave a phrase")]
#[usage("\"<phrase>\"")]
#[example("\"Hello World!\"")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn vaporwave(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let phrase = args.single_quoted::<String>()?;
    msg.channel_id
        .say(&ctx.http, vaporwave_str(&phrase))
        .await?;
    Ok(())
}

pub fn vaporwave_str(data: &str) -> String {
    data.chars()
        .filter_map(|c| {
            let c = c as u32;
            if (33..=270).contains(&c) {
                std::char::from_u32(c + 65248) // unwrap or c ?
            } else {
                Some(32 as char)
            }
        })
        .collect()
}
