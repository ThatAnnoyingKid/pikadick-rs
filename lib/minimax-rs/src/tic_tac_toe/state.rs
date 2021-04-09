use super::{
    TicTacToeIter,
    TicTacToeTeam,
    WinType,
    WinnerInfo,
};
use std::convert::TryInto;

/// A Tic-Tac-Toe game state
///
/// `0xFFFF > (3 ^ 9)`, so `u16` is good.
/// Tic-Tac-Toe states are stored like this in a `u16`:
/// `t * 3.pow(i)` where
/// i is the index of the tic-tac-toe board.
/// t is 0 <= t < 3.
/// t == 0 is empty.
/// t == 1 is X.
/// t == 2 is O.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TicTacToeState(u16);

impl TicTacToeState {
    /// Make a new starting Tic-Tac-Toe board state
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    /// Iterate over the tiles
    #[inline]
    pub fn iter(self) -> TicTacToeIter {
        TicTacToeIter::new(self.0)
    }

    /// Get the tile at the index.
    ///
    /// The index is valid from -1 < index < 9.
    ///
    /// # Panics
    /// Panics if the index is invalid.
    #[inline]
    pub fn at(self, index: u8) -> Option<TicTacToeTeam> {
        self.try_at(index).expect("invalid board index")
    }

    /// Get the tile at the index, returning None on error.
    ///
    /// The index is valid from -1 < index < 9.
    #[inline]
    pub fn try_at(self, index: u8) -> Option<Option<TicTacToeTeam>> {
        self.iter().nth(index.into())
    }

    /// Set the tile at the index, returning the old tile. Panics on failure.
    ///
    /// The index is valid from -1 < index < 9.
    #[inline]
    pub fn set(&mut self, index: u8, team: Option<TicTacToeTeam>) -> Option<TicTacToeTeam> {
        let old = self.at(index);

        let team = match team {
            None => 0,
            Some(TicTacToeTeam::X) => 1,
            Some(TicTacToeTeam::O) => 2,
        };

        let new_entry = team * 3u16.pow(u32::from(index));
        self.0 += new_entry;

        old
    }

    /// Set the tile at the index, returning the old tile if successful.
    ///
    /// The index is valid from -1 < index < 9.
    #[inline]
    pub fn try_set(
        &mut self,
        index: u8,
        team: Option<TicTacToeTeam>,
    ) -> Option<Option<TicTacToeTeam>> {
        let old = self.try_at(index)?;

        let team = match team {
            None => 0,
            Some(TicTacToeTeam::X) => 1,
            Some(TicTacToeTeam::O) => 2,
        };

        let new_entry = team * 3u16.pow(u32::from(index));
        self.0 += new_entry;

        Some(old)
    }

    /// Get whos turn it is.
    pub fn get_team_turn(self) -> TicTacToeTeam {
        let mut x_num = 0;
        let mut o_num = 0;
        for team in self.iter().filter_map(std::convert::identity) {
            match team {
                TicTacToeTeam::X => x_num += 1,
                TicTacToeTeam::O => o_num += 1,
            }
        }

        if x_num > o_num {
            TicTacToeTeam::O
        } else {
            TicTacToeTeam::X
        }
    }

    /// Get the winning team, if there is one.
    pub fn get_winning_team(self) -> Option<TicTacToeTeam> {
        self.get_winning_info().map(|info| info.team)
    }

    /// Utility function for testing whether 3 indexes are populated and are the same team.
    fn check_indexes(
        self,
        one_index: u8,
        two_index: u8,
        three_index: u8,
        win_type: WinType,
    ) -> Option<WinnerInfo> {
        let one = self.at(one_index)?;
        let two = self.at(two_index)?;
        let three = self.at(three_index)?;
        if one == two && two == three {
            return Some(WinnerInfo {
                team: one,
                win_type,
                tile_indexes: [one_index, two_index, three_index],
            });
        }

        None
    }

    /// Get the winning info, if there was a winner.
    pub fn get_winning_info(self) -> Option<WinnerInfo> {
        // Horizontal 1
        if let Some(winner_info) = self.check_indexes(0, 1, 2, WinType::Horizontal) {
            return Some(winner_info);
        }

        // Horizontal 2
        if let Some(winner_info) = self.check_indexes(3, 4, 5, WinType::Horizontal) {
            return Some(winner_info);
        }

        // Horizontal 3
        if let Some(winner_info) = self.check_indexes(6, 7, 8, WinType::Horizontal) {
            return Some(winner_info);
        }

        // Vertical 1
        if let Some(winner_info) = self.check_indexes(0, 3, 6, WinType::Vertical) {
            return Some(winner_info);
        }

        // Vertical 2
        if let Some(winner_info) = self.check_indexes(1, 4, 7, WinType::Vertical) {
            return Some(winner_info);
        }

        // Vertical 3
        if let Some(winner_info) = self.check_indexes(2, 5, 8, WinType::Vertical) {
            return Some(winner_info);
        }

        // Diagonal
        if let Some(winner_info) = self.check_indexes(0, 4, 8, WinType::Diagonal) {
            return Some(winner_info);
        }

        // Anti Diagonal
        if let Some(winner_info) = self.check_indexes(2, 4, 6, WinType::AntiDiagonal) {
            return Some(winner_info);
        }

        None
    }

    /// Check if a game state is a tie
    pub fn is_tie(self) -> bool {
        self.iter().all(|s| s.is_some())
    }

    /// Get the child states for this game
    pub fn get_child_states(self) -> Vec<Self> {
        let team = self.get_team_turn();
        let mut states = Vec::with_capacity(9);

        for (i, tile_team) in self.iter().enumerate() {
            if tile_team.is_none() {
                let i_u8 = i.try_into().expect("could not fit index in a `u8`");
                let mut new_state = self;
                new_state.set(i_u8, Some(team));

                states.push(new_state);
            }
        }

        states
    }

    /// Convert this into a [`u16`].
    pub fn into_u16(self) -> u16 {
        self.0
    }

    /// Convert a [`u16`] into this.
    // TODO: Should i make sure only valid states are accepted?
    pub fn from_u16(data: u16) -> Self {
        Self(data)
    }
}

impl Default for TicTacToeState {
    fn default() -> Self {
        Self::new()
    }
}
