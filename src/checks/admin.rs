use serenity::{
    client::Context,
    framework::standard::{
        Args,
        CommandOptions,
        Reason,
        macros::check,
    },
    model::prelude::*,
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
            let guild_channel = match guild.channels.get(&msg.channel_id) {
                Some(channel) => channel,
                None => return Err(Reason::Unknown),
            };

            let perms = guild.user_permissions_in(guild_channel, &member);

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
