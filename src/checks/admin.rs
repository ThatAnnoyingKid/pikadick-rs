use crate::BotContext;
use pikadick_slash_framework::{
    BoxFuture,
    Command,
    Reason as SlashReason,
};
use serenity::{
    client::Context,
    framework::standard::{
        macros::check,
        Args,
        CommandOptions,
        Reason,
    },
    model::prelude::*,
};
use tracing::{
    error,
    warn,
};
use twilight_model::application::interaction::{
    application_command::CommandData,
    Interaction,
};

#[check]
#[name("Admin")]
pub async fn admin_check(
    ctx: &Context,
    msg: &Message,
    _args: &mut Args,
    _opts: &CommandOptions,
) -> Result<(), Reason> {
    if let Some(guild_id) = msg.guild_id {
        let member = match msg.member(ctx).await {
            Ok(member) => member,
            Err(e) => {
                return Err(Reason::User(format!("failed to fetch member info: {}", e)));
            }
        };

        if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
            let channel = match guild.channels.get(&msg.channel_id) {
                Some(channel) => channel,
                None => return Err(Reason::Unknown),
            };

            let guild_channel = match channel {
                Channel::Guild(channel) => channel,
                _ => {
                    return Err(Reason::Unknown);
                }
            };

            let perms = match guild.user_permissions_in(guild_channel, &member) {
                Ok(perms) => perms,
                Err(e) => {
                    error!(
                        "error getting permissions for user {} in channel {}: {}",
                        member.user.id,
                        channel.id(),
                        e
                    );
                    return Err(Reason::Unknown);
                }
            };

            if perms.administrator() {
                Ok(())
            } else {
                Err(Reason::User("not admin".to_string()))
            }
        } else {
            Err(Reason::User("guild not in cache".to_string()))
        }
    } else {
        // User is probably in a DM.
        Ok(())
    }
}

/// Ensure a user is admin
pub fn create_slash_check<'a>(
    _client_data: &'a BotContext,
    interaction: &'a Interaction,
    _command_data: &'a CommandData,
    _command: &'a Command<BotContext>,
) -> BoxFuture<'a, Result<(), SlashReason>> {
    Box::pin(async move {
        match interaction.guild_id {
            Some(id) => id,
            None => {
                // Let's not care about dms for now.
                // They'll probably need special handling anyways.
                // This will also probably only be useful in Group DMs,
                // which I don't think bots can participate in anyways.
                return Ok(());
            }
        };

        match interaction
            .member
            .as_ref()
            .and_then(|member| member.permissions)
        {
            Some(permissions) => {
                if permissions.contains(twilight_model::guild::Permissions::ADMINISTRATOR) {
                    Ok(())
                } else {
                    Err(SlashReason::new_user("Not Admin.".to_string()))
                }
            }
            None => {
                // Failed to get member permissions.
                // I don't think this matters since I think this is only absent in dms.
                warn!("failed to get member permissions");
                Err(SlashReason::new_unknown())
            }
        }
    })
}
