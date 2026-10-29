use crate::{
    PoiseContext,
    PoiseError,
};

#[poise::command(slash_command, description_localized("en-US", "Respond with pong."))]
pub async fn ping(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.reply("Pong").await?;
    Ok(())
}
