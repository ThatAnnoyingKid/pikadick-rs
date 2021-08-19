use crate::checks::ENABLED_CHECK;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};

#[command]
#[description("Respond with pong")]
#[checks(Enabled)]
#[bucket("default")]
async fn ping(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.channel_id.say(&ctx.http, "pong").await?;
    Ok(())
}
