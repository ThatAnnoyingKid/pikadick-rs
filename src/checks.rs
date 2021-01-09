pub mod enabled;

pub use self::enabled::*;
use serenity::{
    framework::standard::{
        macros::check,
        Args,
        CommandOptions,
        Reason,
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
) -> Result<(), Reason> {
    if let Some(member) = &msg.member {
        for role in member.roles.iter() {
            if role
                .to_role_cached(&ctx.cache)
                .await
                .map_or(false, |r| r.has_permission(Permissions::ADMINISTRATOR))
            {
                return Ok(());
            }
        }
    }

    Err(Reason::User("Not Admin.".to_string()))
}
