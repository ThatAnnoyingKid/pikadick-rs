use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context;
use poise::CreateReply;
use serenity::builder::CreateAttachment;
use tracing::error;

#[poise::command(
    slash_command,
    description_localized("en-US", "Print the current Tic-Tac-Toe board.")
)]
pub async fn board(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let tic_tac_toe_data = &ctx.data().tic_tac_toe_data;

    let guild_id = ctx.guild_id();
    let author_id = ctx.author().id;

    match tic_tac_toe_data.get_game(guild_id, author_id).await {
        Ok(Some(game)) => {
            let file = match tic_tac_toe_data
                .render_board(game.board)
                .await
                .context("Failed to render Tic-Tac-Toe board")
            {
                Ok(file) => {
                    CreateAttachment::bytes(file, format!("ttt-{}.png", game.board.encode_u16()))
                }
                Err(error) => {
                    error!("{error:?}");
                    ctx.reply(format!("{error:?}")).await?;
                    return Ok(());
                }
            };
            let create_reply = CreateReply::new().attachment(file);

            ctx.send(create_reply).await?;
        }
        Ok(None) => {
            let response = "Failed to print board as you have no games in this server.";
            ctx.reply(response).await?;
        }
        Err(error) => {
            error!("{error:?}");
            ctx.reply("Database error").await?;
        }
    }

    Ok(())
}
