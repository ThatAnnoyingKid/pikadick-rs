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
    state: tic_tac_toe::Board,
    x_player: TicTacToePlayer,
    o_player: TicTacToePlayer,
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
