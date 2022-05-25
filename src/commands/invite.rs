use crate::checks::ENABLED_CHECK;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::{
        channel::Message,
        permissions::Permissions,
    },
};

#[command]
#[description("Get an invite link for this bot")]
#[checks(Enabled)]
#[bucket("default")]
pub async fn invite(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let user = ctx.http.get_current_user().await?;
    let link = user.invite_url(&ctx.http, Permissions::empty()).await?;
    msg.channel_id.say(&ctx.http, &link).await?;
    Ok(())
}
