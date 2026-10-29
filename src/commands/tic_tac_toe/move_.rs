use crate::{
    PoiseContext,
    PoiseError,
    database::{
        TicTacToeTryMoveError,
        TicTacToeTryMoveResponse,
    },
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
    rename = "move",
    description_localized("en-US", "Start a game of Tic-Tac-Toe.")
)]
pub async fn move_(
    ctx: PoiseContext<'_>,
    #[description_localized("en-US", "The number of the move.")]
    #[min = 1]
    #[max = 9]
    number: u8,
) -> Result<(), PoiseError> {
    let tic_tac_toe_data = &ctx.data().tic_tac_toe_data;

    let guild_id = ctx.guild_id();
    let author = ctx.author();
    let author_id = author.id;

    if !(1..=9).contains(&number) {
        let response = format!(
            "Your move number must be between 1 and 9 {}",
            author_id.mention()
        );
        ctx.reply(response).await?;
        return Ok(());
    }
    let move_index = number - 1;

    match tic_tac_toe_data
        .try_move(guild_id, author_id, move_index)
        .await
    {
        Ok(TicTacToeTryMoveResponse::Winner {
            game,
            winner,
            loser,
        }) => {
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
            let content = format!(
                "{} has triumphed over {} in Tic-Tac-Toe",
                winner.mention(),
                loser.mention(),
            );

            let create_reply = CreateReply::new().content(content).attachment(file);
            ctx.send(create_reply).await?;
        }
        Ok(TicTacToeTryMoveResponse::Tie { game }) => {
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
            let content = format!(
                "{} has tied with {} in Tic-Tac-Toe",
                game.get_player(tic_tac_toe::Team::X).mention(),
                game.get_player(tic_tac_toe::Team::O).mention(),
            );
            let create_reply = CreateReply::new().content(content).attachment(file);
            ctx.send(create_reply).await?;
        }
        Ok(TicTacToeTryMoveResponse::NextTurn { game }) => {
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
            let content = format!("Your turn {}", game.get_player_turn().mention());
            let create_reply = CreateReply::new().content(content).attachment(file);
            ctx.send(create_reply).await?;
        }
        Err(TicTacToeTryMoveError::InvalidTurn) => {
            let response = "It is not your turn. Please wait for your opponent to finish.";
            ctx.reply(response).await?;
        }
        Err(TicTacToeTryMoveError::InvalidMove) => {
            let response = format!(
                "Invalid move {}. Please choose one of the available squares.\n",
                author_id.mention(),
            );
            ctx.reply(response).await?;
        }
        Err(TicTacToeTryMoveError::NotInAGame) => {
            let response = "No games in progress. Make one with `tic-tac-toe play`.";
            ctx.reply(response).await?;
        }
        Err(TicTacToeTryMoveError::Database(error)) => {
            error!("{error:?}");
            ctx.reply("Database error").await?;
        }
    }

    Ok(())
}
