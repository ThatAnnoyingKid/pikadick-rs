pub mod enabled;

pub use self::enabled::*;
use serenity::{
    framework::standard::{
        macros::check,
        Args,
        CheckResult,
        CommandOptions,
    },
    model::prelude::*,
    prelude::*,
};

#[check]
#[name("Admin")]
pub async fn admin_check(
    ctx: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> CheckResult {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => {
            return true.into();
        }
    };

    let user_permissions = match ctx
        .cache
        .guild_field(guild_id, |guild| guild.member_permissions(msg.author.id))
        .await
    {
        Some(p) => p,
        None => {
            // If it has an associated guild in the message but isn't in the cache, return false to be safe?
            return false.into();
        }
    };

    user_permissions.administrator().into()
}
