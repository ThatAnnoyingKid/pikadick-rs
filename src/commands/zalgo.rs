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
use zalgo::{
    RandOrStatic,
    Zalgoifier,
};

#[command]
#[description("Zalgoify a phrase")]
#[usage("\"<phrase>\"<Max Length>")]
#[example("\"Hello World!\" 50")]
#[min_args(1)]
#[max_args(2)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn zalgo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let input: String = args.single_quoted()?;
    let input_max = args.single().unwrap_or(2000);

    let input_len = input.chars().count();
    let total = (input_max as f32 - input_len as f32) / input_len as f32;
    let max = (total / 3.0) as usize;

    if max == 0 {
        msg.channel_id
            .say(
                &ctx.http,
                "The phrase cannot be zalgoified within the given limits",
            )
            .await?;
        return Ok(());
    }

    let output = {
        let mut zalgoifier = Zalgoifier::new();
        zalgoifier.set_up(RandOrStatic::Static(max));
        zalgoifier.set_down(RandOrStatic::Static(max));
        zalgoifier.set_mid(RandOrStatic::Static(max));
        zalgoifier.zalgoify(&input)
    };

    msg.channel_id.say(&ctx.http, &output).await?;

    Ok(())
}
