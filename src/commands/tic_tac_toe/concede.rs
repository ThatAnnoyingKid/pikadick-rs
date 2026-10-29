use crate::{
    PoiseContext,
    PoiseError,
    database::model::TicTacToePlayer,
};
use anyhow::Context;
use poise::CreateReply;
use serenity::{
    builder::CreateAttachment,
    prelude::Mentionable,
};
use tracing::error;

#[poise::command(
    slash_command,
    description_localized("en-US", "Concede a game of Tic-Tac-Toe.")
)]
pub async fn concede(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let tic_tac_toe_data = &ctx.data().tic_tac_toe_data;

    let guild_id = ctx.guild_id();
    let author_id = ctx.author().id;

    let game = match tic_tac_toe_data.concede_game(guild_id, author_id).await {
        Ok(Some(game)) => game,
        Ok(None) => {
            let response = "Failed to concede as you have no games in this server";
            ctx.reply(response).await?;
            return Ok(());
        }
        Err(error) => {
            error!("{error:?}");
            ctx.reply("Database error").await?;
            return Ok(());
        }
    };

    let opponent = game
        .get_opponent(TicTacToePlayer::User(author_id))
        .expect("Author is not playing the game");

    let file = match tic_tac_toe_data
        .renderer
        .render_board_async(game.board)
        .await
        .context("Failed to render Tic-Tac-Toe board")
    {
        Ok(file) => CreateAttachment::bytes(file, format!("ttt-{}.png", game.board.encode_u16())),
        Err(error) => {
            error!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;
            return Ok(());
        }
    };

    let content = format!(
        "{} has conceded to {}.",
        author_id.mention(),
        opponent.mention()
    );

    let create_reply = CreateReply::new().content(content).attachment(file);
    ctx.send(create_reply).await?;

    Ok(())
}
