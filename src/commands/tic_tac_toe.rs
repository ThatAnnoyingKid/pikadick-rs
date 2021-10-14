mod board;
mod concede;
mod play;
mod renderer;
mod stats;

use self::renderer::Renderer;
pub use self::{
    board::BOARD_COMMAND,
    concede::CONCEDE_COMMAND,
    play::PLAY_COMMAND,
    stats::STATS_COMMAND,
};
use crate::{
    checks::ENABLED_CHECK,
    database::{
        model::TicTacToePlayer,
        TicTacToeTryMoveError,
        TicTacToeTryMoveResponse,
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
    model::{
        channel::Message,
        prelude::*,
    },
};
use std::sync::Arc;
use tracing::error;

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    renderer: Arc<Renderer>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new() -> Self {
        let renderer = Renderer::new().expect("failed to init renderer");

        Self {
            renderer: Arc::new(renderer),
        }
    }
}

impl std::fmt::Debug for TicTacToeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TicTacToeData").finish()
    }
}

impl Default for TicTacToeData {
    fn default() -> Self {
        Self::new()
    }
}

impl TicTacToePlayer {
    /// Get the "mention" for a user.
    ///
    /// Computer is "computer" and users are mentioned.
    pub fn mention(self) -> GamePlayerMention {
        GamePlayerMention(self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GamePlayerMention(TicTacToePlayer);

impl std::fmt::Display for GamePlayerMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            TicTacToePlayer::Computer => "computer".fmt(f),
            TicTacToePlayer::User(user_id) => user_id.mention().fmt(f),
        }
    }
}

#[command("tic-tac-toe")]
#[aliases("ttt")]
#[sub_commands("play", "concede", "board", "stats")]
#[description("Play a game of Tic-Tac-Toe")]
#[usage("<move #>")]
#[example("0")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn tic_tac_toe(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let mut move_index = match args.trimmed().single::<u8>() {
        Ok(num) => num,
        Err(e) => {
            let response = format!("That move is not a number: {}\nUse `tic-tac-toe play <computer/@user> <X/O> to start a game.`", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    if !(1..=9).contains(&move_index) {
        let response = format!(
            "Your move number must be between 1 and 9 {}",
            author_id.mention()
        );
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    move_index -= 1;

    match db
        .try_tic_tac_toe_move(guild_id.into(), author_id.into(), move_index)
        .await
    {
        Ok(TicTacToeTryMoveResponse::Winner {
            game,
            winner,
            loser,
        }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
            let content = format!(
                "{} has triumphed over {} in Tic-Tac-Toe",
                winner.mention(),
                loser.mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TicTacToeTryMoveResponse::Tie { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
            let content = format!(
                "{} has tied with {} in Tic-Tac-Toe",
                game.get_player(tic_tac_toe::Team::X).mention(),
                game.get_player(tic_tac_toe::Team::O).mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TicTacToeTryMoveResponse::NextTurn { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
            let content = format!("Your turn {}", game.get_player_turn().mention());
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Err(TicTacToeTryMoveError::InvalidTurn) => {
            let response = "It is not your turn. Please wait for your opponent to finish.";
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TicTacToeTryMoveError::InvalidMove) => {
            let response = format!(
                "Invalid move {}. Please choose one of the available squares.\n",
                author_id.mention(),
            );
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TicTacToeTryMoveError::NotInAGame) => {
            let response =
                "No games in progress. Make one with `tic-tac-toe play <computer/@user> <X/O>`.";
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TicTacToeTryMoveError::Database(e)) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, "database error").await?;
        }
    }

    Ok(())
}
