mod board;
mod concede;
mod data;
mod play;
mod renderer;

use self::renderer::Renderer;
pub use self::{
    board::BOARD_COMMAND,
    concede::CONCEDE_COMMAND,
    data::TicTacToeData,
    play::PLAY_COMMAND,
};
use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use log::error;
use minimax::{
    tic_tac_toe::TicTacToeState,
    TicTacToeTeam,
};
use parking_lot::Mutex;
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
    utils::parse_username,
};
use std::{
    str::FromStr,
    sync::Arc,
};

/// Error that may occur while creating a game.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum CreateGameError {
    /// The author is in a game
    #[error("the author is in a game")]
    AuthorInGame,

    /// The opponent is in a game
    #[error("the opponent is in a game")]
    OpponentInGame,
}

/// Error that may occur while performing a game move.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum TryMoveError {
    /// It is not the user's turn
    #[error("not the user's turn to move")]
    InvalidTurn,

    /// The move is invalid
    #[error("the move is not valid")]
    InvalidMove,
}

/// The response for making a move.
#[derive(Debug, Copy, Clone)]
pub enum TryMoveResponse {
    Winner {
        game: GameState,
        winner: GamePlayer,
        loser: GamePlayer,
    },
    Tie {
        game: GameState,
    },
    NextTurn {
        game: GameState,
    },
}

/// A [`GuildId`]/[`UserId`] key to a [`GameState`].
pub type GameStateKey = (Option<GuildId>, UserId);

/// A [`GameState`] that is wrapped in a mutex and sharable via a rc'ed ptr.
pub type ShareGameState = Arc<Mutex<GameState>>;

/// A Tic-Tac-Toe game.
#[derive(Debug, Copy, Clone)]
pub struct GameState {
    /// The Game state
    state: TicTacToeState,

    /// The X player
    x_player: GamePlayer,

    /// The O player
    o_player: GamePlayer,
}

impl GameState {
    /// Iterate over all [`GamePlayer`]s.
    ///
    /// Order is X player, O player.
    /// This will include computer players.
    /// Convert players into [`UserId`]s and filter if you want human players.
    pub fn iter_players(&self) -> impl Iterator<Item = GamePlayer> + '_ {
        let mut count = 0;
        std::iter::from_fn(move || {
            let ret = match count {
                0 => self.x_player,
                1 => self.o_player,
                _c => return None,
            };
            count += 1;
            Some(ret)
        })
    }

    /// Get whos turn it is
    pub fn get_team_turn(&self) -> TicTacToeTeam {
        self.state.get_team_turn()
    }

    /// Get the player whos turn it is
    pub fn get_player_turn(&self) -> GamePlayer {
        let turn = self.get_team_turn();
        match turn {
            TicTacToeTeam::X => self.x_player,
            TicTacToeTeam::O => self.o_player,
        }
    }

    /// Try to make a move. Returns true if successful.
    pub fn try_move(&mut self, index: u8, team: TicTacToeTeam) -> bool {
        if self.state.at(index).is_some() {
            false
        } else {
            self.state.set(index, Some(team));
            true
        }
    }

    /// Get the opponent of the given user in this [`GameState`].
    pub fn get_opponent(&self, player: GamePlayer) -> Option<GamePlayer> {
        match (player == self.x_player, player == self.o_player) {
            (false, false) => None,
            (false, true) => Some(self.x_player),
            (true, false) => Some(self.o_player),
            (true, true) => Some(player), // Player is playing themselves
        }
    }

    /// Get the player for the given team.
    pub fn get_player(&self, team: TicTacToeTeam) -> GamePlayer {
        match team {
            TicTacToeTeam::X => self.x_player,
            TicTacToeTeam::O => self.o_player,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InvalidGamePlayer;

impl std::fmt::Display for InvalidGamePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "invalid player".fmt(f)
    }
}

/// A player of Tic-Tac-Toe
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GamePlayer {
    /// User
    Computer,

    /// A User
    User(UserId),
}

impl GamePlayer {
    /// Try to convert this into a [`UserId`].
    pub fn into_user_id(self) -> Option<UserId> {
        match self {
            Self::User(id) => Some(id),
            _ => None,
        }
    }

    /// Check if this player is a computer
    pub fn is_computer(self) -> bool {
        matches!(self, Self::Computer)
    }

    /// Get the "mention" for a user.
    ///
    /// Computer is "computer" and users are mentioned.
    pub fn mention(self) -> GamePlayerMention {
        GamePlayerMention(self)
    }
}

impl FromStr for GamePlayer {
    type Err = InvalidGamePlayer;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        if data.eq_ignore_ascii_case("computer") {
            return Ok(Self::Computer);
        }

        parse_username(data)
            .map(|id| Self::User(UserId(id)))
            .ok_or(InvalidGamePlayer)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GamePlayerMention(GamePlayer);

impl std::fmt::Display for GamePlayerMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            GamePlayer::Computer => "computer".fmt(f),
            GamePlayer::User(user_id) => user_id.mention().fmt(f),
        }
    }
}

#[command("tic-tac-toe")]
#[aliases("ttt")]
#[sub_commands("play", "concede", "board")]
#[description("Play a game of Tic-Tac-Toe")]
#[usage("<move #>")]
#[example("0")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
pub async fn tic_tac_toe(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let mut move_number = match args.trimmed().single::<u8>() {
        Ok(num) => num,
        Err(e) => {
            let response = format!("That move is not a number: {}\nUse `tic-tac-toe play <computer/@user> <X/O> to start a game.`", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    if !(1..=9).contains(&move_number) {
        let response = format!(
            "Your move number must be between 1 and 9 {}",
            author_id.mention()
        );
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    move_number -= 1;

    let game_state = match tic_tac_toe_data.get_game_state(&(guild_id, author_id)) {
        Some(game_state) => game_state,
        None => {
            let response =
                "No games in progress. Make one with `tic-tac-toe play <computer/@user> <X/O>`.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    match tic_tac_toe_data.try_move(game_state, guild_id, author_id, move_number) {
        Ok(TryMoveResponse::Winner {
            game,
            winner,
            loser,
        }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state.into_u16()),
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
        Ok(TryMoveResponse::Tie { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state.into_u16()),
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
                game.get_player(TicTacToeTeam::X).mention(),
                game.get_player(TicTacToeTeam::O).mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TryMoveResponse::NextTurn { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state.into_u16()),
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
        Err(TryMoveError::InvalidTurn) => {
            let response = "It is not your turn. Please wait for your opponent to finish.";
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TryMoveError::InvalidMove) => {
            let response = format!(
                "Invalid move {}. Please choose one of the available squares.\n",
                author_id.mention(),
            );
            msg.channel_id.say(&ctx.http, response).await?;
        }
    }

    Ok(())
}
