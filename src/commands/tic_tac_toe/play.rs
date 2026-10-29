use crate::{
    PoiseContext,
    PoiseError,
    database::{
        TicTacToeCreateGameError,
        model::TicTacToePlayer,
    },
};
use anyhow::Context as _;
use poise::CreateReply;
use serenity::{
    builder::CreateAttachment,
    model::id::UserId,
    prelude::Mentionable,
};
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
enum TeamChoice {
    #[name = "X"]
    X,

    #[name = "O"]
    O,
}

#[poise::command(
    slash_command,
    description_localized("en-US", "Start a game of Tic-Tac-Toe.")
)]
pub async fn play(
    ctx: PoiseContext<'_>,
    #[description_localized("en-US", "The user you want to face. Omit to face the computer.")]
    opponent: Option<UserId>,
    #[description_localized("en-US", "The team you want to play as, X or O. Defaults to X.")] team: Option<
        TeamChoice,
    >,
) -> Result<(), PoiseError> {
    let author = ctx.author();
    let author_id = author.id;
    let author_team = match team {
        Some(TeamChoice::X) | None => tic_tac_toe::Team::X,
        Some(TeamChoice::O) => tic_tac_toe::Team::O,
    };
    let opponent = match opponent {
        Some(user_id) => TicTacToePlayer::User(user_id),
        None => TicTacToePlayer::Computer,
    };
    let create_game_result = ctx
        .data()
        .tic_tac_toe_data
        .create_game(ctx.guild_id(), author_id, author_team, opponent)
        .await;
    let game = match create_game_result {
        Ok(game) => game,
        Err(TicTacToeCreateGameError::AuthorInGame) => {
            let response = "Finish your current game in this server before starting a new one. Use `tic-tac-toe concede` to end your current game.";
            ctx.reply(response).await?;
            return Ok(());
        }
        Err(TicTacToeCreateGameError::OpponentInGame) => {
            let response = "Your opponent is currently in another game in this server. Wait for them to finish.";
            ctx.reply(response).await?;
            return Ok(());
        }
        Err(TicTacToeCreateGameError::Database(error)) => {
            error!("{error:?}");
            ctx.reply("Database error").await?;
            return Ok(());
        }
    };

    let game_board = game.board;
    let user = if let TicTacToePlayer::User(opponent_id) = opponent {
        // Cannot be a computer here as there are at least 2 human players at this point
        if author_team == tic_tac_toe::Team::X {
            author_id
        } else {
            opponent_id
        }
    } else {
        // The opponent is not a user, so it is a computer.
        // We already calculated the move and updated if the computer is X.
        // All that's left is to @author and print the board state.
        author_id
    };

    let file = match ctx
        .data()
        .tic_tac_toe_data
        .render_board(game_board)
        .await
        .context("Failed to render Tic-Tac-Toe board")
    {
        Ok(file) => CreateAttachment::bytes(file, format!("{}.png", game_board.encode_u16())),
        Err(error) => {
            error!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;
            return Ok(());
        }
    };

    let create_reply = CreateReply::new()
        .content(format!("Game created! Your turn {}", user.mention()))
        .attachment(file);

    ctx.send(create_reply).await?;

    Ok(())
}
