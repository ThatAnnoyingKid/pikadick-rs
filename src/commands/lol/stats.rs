use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use tracing::warn;

#[poise::command(slash_command, description_localized("en-US", "Get lol stats."))]
pub async fn stats(ctx: PoiseContext<'_>, user: String) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let result = ctx
        .data()
        .ugg_client
        .client
        .search_users(&user)
        .await
        .context("Failed to search for users");
    let users = match result {
        Ok(users) => users,
        Err(error) => {
            warn!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;

            return Ok(());
        }
    };

    ctx.reply(format!("{users:#?}")).await?;

    Ok(())
}
