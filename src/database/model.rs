mod jiff_timestamp_wrapper;
mod option_guild_id_string;
mod rule34_post;
mod rule34_query_stat;
mod serenity_user_id_wrapper;
mod tiktok_embed_flags;

pub use self::{
    jiff_timestamp_wrapper::JiffTimestampWrapper,
    option_guild_id_string::OptionGuildIdString,
    rule34_post::Rule34Post,
    rule34_query_stat::Rule34QueryStat,
    serenity_user_id_wrapper::SerenityUserIdWrapper,
    tiktok_embed_flags::TikTokEmbedFlags,
};
use nd_async_rusqlite::rusqlite::{
    Result as RusqliteResult,
    Row as RusqliteRow,
    ToSql,
    types::{
        FromSql,
        FromSqlError,
        FromSqlResult,
        ToSqlOutput,
        ValueRef,
    },
};
use serenity::{
    model::prelude::*,
    utils::parse_user_mention,
};
use std::{
    borrow::Cow,
    num::NonZeroU64,
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
        if index >= tic_tac_toe::NUM_TILES || self.board.get(index).is_some() {
            false
        } else {
            self.board = self.board.set(index, Some(team));
            true
        }
    }

    /// Get the opponent of the given user in this [`TicTacToeGame`].
    pub fn get_opponent(&self, player: TicTacToePlayer) -> Option<TicTacToePlayer> {
        match (player == self.x_player, player == self.o_player) {
            (false, false) => None,
            (false, true) => Some(self.x_player),
            (true, false) => Some(self.o_player),
            (true, true) => Some(player), // Player is playing themselves
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicTacToePlayerParseError(std::num::ParseIntError);

impl std::fmt::Display for TicTacToePlayerParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "Invalid player".fmt(f)
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
        } else if let Some(user_id) = parse_user_mention(input) {
            Ok(Self::User(user_id))
        } else {
            let user_id: NonZeroU64 = input.parse().map_err(TicTacToePlayerParseError)?;
            Ok(Self::User(UserId::from(user_id)))
        }
    }
}

impl From<UserId> for TicTacToePlayer {
    fn from(user_id: UserId) -> Self {
        Self::User(user_id)
    }
}

impl ToSql for TicTacToePlayer {
    fn to_sql(&self) -> RusqliteResult<ToSqlOutput<'_>> {
        match self {
            Self::Computer => Ok(ToSqlOutput::Borrowed(ValueRef::Null)),
            Self::User(id) => Ok(ToSqlOutput::Borrowed(ValueRef::Integer(i64::from(*id)))),
        }
    }
}

impl FromSql for TicTacToePlayer {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(int) => {
                let int_u64 = u64::from_ne_bytes(int.to_ne_bytes());
                let user_id =
                    UserId::from(NonZeroU64::new(int_u64).ok_or(FromSqlError::OutOfRange(0))?);
                Ok(Self::User(user_id))
            }
            ValueRef::Null => Ok(Self::Computer),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

/// Tic-Tac-Toe scores
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TicTacToeScore {
    /// Wins
    pub wins: u64,
    /// Losses
    pub losses: u64,
    /// Ties
    pub ties: u64,
    /// The number of times the player has conceded
    pub concedes: u64,
}

/// Top Player Tic-Tac-Toe scores
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TicTacToeTopPlayerScore {
    /// The score
    pub score: i64,
    /// The player
    pub player: UserId,
    /// Wins
    pub wins: u64,
    /// Losses
    pub losses: u64,
    /// Ties
    pub ties: u64,
    /// The number of times the player has conceded
    pub concedes: u64,
}

impl TicTacToeTopPlayerScore {
    /// Parse this from a rusqlite row.
    ///
    /// Data must be in the following order:
    /// 1. score
    /// 2. player
    /// 3. wins
    /// 4. losses
    /// 5. ties
    /// 6. concedes
    pub(crate) fn from_row(row: &RusqliteRow<'_>) -> RusqliteResult<Self> {
        let score = row.get(0)?;
        let SerenityUserIdWrapper(player) = row.get(1)?;
        let wins = row.get(2)?;
        let losses = row.get(3)?;
        let ties = row.get(4)?;
        let concedes = row.get(5)?;

        Ok(TicTacToeTopPlayerScore {
            score,
            player,
            wins,
            losses,
            ties,
            concedes,
        })
    }
}
