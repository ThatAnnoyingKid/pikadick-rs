use crate::checks::ENABLED_CHECK;
use serenity::{
    builder::CreateBotAuthParameters,
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::{
        application::Scope,
        channel::Message,
        permissions::Permissions,
    },
};

#[command]
#[description("Get an invite link for this bot")]
#[checks(Enabled)]
#[bucket("default")]
pub async fn invite(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let permissions = Permissions::empty();
    let link = CreateBotAuthParameters::new()
        .permissions(permissions)
        .auto_client_id(&ctx)
        .await?
        .scopes(&[Scope::Bot])
        .build();

    msg.channel_id.say(&ctx.http, &link).await?;

    Ok(())
}
