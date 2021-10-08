use crate::{
    checks::ENABLED_CHECK,
    database::{
        model::TicTacToePlayer,
        TicTacToeCreateGameError,
    },
    ClientDataKey,
};
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    http::AttachmentType,
    model::prelude::*,
};
use tracing::error;

#[command]
#[description("Start a game of Tic-Tac-Toe")]
#[usage("<computer OR @user, X OR O>")]
#[example("computer X")]
#[min_args(2)]
#[max_args(2)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let opponent: TicTacToePlayer = match args.trimmed().single() {
        Ok(player) => player,
        Err(e) => {
            let response = format!(
                "Invalid opponent. Choose 'computer' or '@user'. Error: {}",
                e
            );
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let author_team: tic_tac_toe::Team = match args.trimmed().single() {
        Ok(team) => team,
        Err(e) => {
            let response = format!("Invalid team. Choose 'X' or 'O'. Error: {}", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let author_id = msg.author.id;
    let guild_id = msg.guild_id;

    let (_game_id, game) = match db
        .create_tic_tac_toe_game(guild_id, author_id, author_team, opponent)
        .await
    {
        Ok(game) => game,
        Err(TicTacToeCreateGameError::AuthorInGame) => {
            let response = "Finish your current game in this server before starting a new one. Use `tic-tac-toe concede` to end your current game.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
        Err(TicTacToeCreateGameError::OpponentInGame) => {
            let response = "Your opponent is currently in another game in this server. Wait for them to finish.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
        Err(TicTacToeCreateGameError::Database(e)) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, "database error").await?;
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

    let file = match tic_tac_toe_data
        .renderer
        .render_board_async(game_board)
        .await
    {
        Ok(file) => AttachmentType::Bytes {
            data: file.into(),
            filename: format!("{}.png", game_board.encode_u16()),
        },
        Err(e) => {
            error!("Failed to render Tic-Tac-Toe board: {}", e);
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Failed to render Tic-Tac-Toe board: {}", e),
                )
                .await?;
            return Ok(());
        }
    };

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.content(format!("Game created! Your turn {}", user.mention()))
                .add_file(file)
        })
        .await?;

    Ok(())
}
