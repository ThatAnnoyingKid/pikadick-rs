use serenity::{
    model::prelude::*,
    utils::parse_username,
};
use std::{
    borrow::Cow,
    str::FromStr,
};

/// A Tic-Tac-Toe game
#[derive(Debug, Copy, Clone)]
pub struct TicTacToeGame {
    /// The game board
    pub board: tic_tac_toe::Board,
    /// The x player
    pub x_player: TicTacToePlayer,
    /// The o player
    pub o_player: TicTacToePlayer,
}

impl TicTacToeGame {
    /// Make a new [`TicTacToeGame`].
    pub(super) fn new(x_player: TicTacToePlayer, o_player: TicTacToePlayer) -> Self {
        Self {
            board: Default::default(),
            x_player,
            o_player,
        }
    }

    /// Get whos turn it is
    pub fn get_team_turn(&self) -> tic_tac_toe::Team {
        self.board.get_turn()
    }

    /// Get the player for the given team.
    pub fn get_player(&self, team: tic_tac_toe::Team) -> TicTacToePlayer {
        match team {
            tic_tac_toe::Team::X => self.x_player,
            tic_tac_toe::Team::O => self.o_player,
        }
    }

    /// Get the player whos turn it is
    pub fn get_player_turn(&self) -> TicTacToePlayer {
        self.get_player(self.get_team_turn())
    }

    /// Try to make a move.
    ///
    /// # Returns
    /// Returns true if successful.
    pub fn try_move(&mut self, index: u8, team: tic_tac_toe::Team) -> bool {
        if index >= tic_tac_toe::NUM_TILES {
            false
        } else if self.board.get(index).is_some() {
            false
        } else {
            self.board = self.board.set(index, Some(team));
            true
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicTacToePlayerParseError(std::num::ParseIntError);

impl std::fmt::Display for TicTacToePlayerParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "invalid player".fmt(f)
    }
}

impl std::error::Error for TicTacToePlayerParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// A player of Tic-Tac-Toe
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TicTacToePlayer {
    /// AI player
    Computer,

    /// Another user
    User(UserId),
}

impl TicTacToePlayer {
    /// Check if this player is a computer
    pub fn is_computer(self) -> bool {
        matches!(self, Self::Computer)
    }

    /// Check if this player is a user
    pub fn is_user(self) -> bool {
        matches!(self, Self::User(_))
    }

    /// Extract the user id if this is a user
    pub fn get_user(self) -> Option<UserId> {
        match self {
            Self::Computer => None,
            Self::User(user_id) => Some(user_id),
        }
    }
}

impl From<TicTacToePlayer> for Cow<'static, str> {
    fn from(player: TicTacToePlayer) -> Self {
        match player {
            TicTacToePlayer::Computer => "computer".into(),
            TicTacToePlayer::User(id) => id.to_string().into(),
        }
    }
}

impl FromStr for TicTacToePlayer {
    type Err = TicTacToePlayerParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.eq_ignore_ascii_case("computer") {
            Ok(Self::Computer)
        } else if let Some(user_id) = parse_username(input) {
            Ok(Self::User(UserId(user_id)))
        } else {
            Ok(Self::User(UserId(
                input.parse().map_err(TicTacToePlayerParseError)?,
            )))
        }
    }
}
