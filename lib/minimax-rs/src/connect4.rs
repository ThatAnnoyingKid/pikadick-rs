use crate::RuleSet;

/// The number of rows in a connect 4 board
pub const ROWS: u8 = 6;

/// The number of columns in a connect 4 board
pub const COLS: u8 = 7;

/// The number of slots in a connect 4 board
pub const SLOTS: u8 = ROWS * COLS;

/// A Connect 4 Game State
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Connect4State(u128);

impl Connect4State {
    /// Make the starting Connect 4 game state
    pub fn new() -> Self {
        Self(0)
    }

    /// Iterate over the tiles
    #[inline]
    pub fn iter(self) -> Connect4Iter {
        Connect4Iter::new(self.0)
    }

    /// Get the tile at the index.
    ///
    /// The index is valid from -1 < index < 42.
    ///
    /// # Panics
    /// Panics if the index is invalid.
    #[inline]
    pub fn at(self, index: u8) -> Option<Connect4Team> {
        self.try_at(index).expect("invalid board index")
    }

    /// Get the tile at the index, returning None on error.
    ///
    /// The index is valid from -1 < index < 42.
    #[inline]
    pub fn try_at(self, index: u8) -> Option<Option<Connect4Team>> {
        self.iter().nth(index.into())
    }

    /// Drop the piece at the column.
    ///
    /// The piece is "dropped" so that it reaches the lowest point of the column possible.
    /// The index is valid from -1 < index < [`COLS`].
    /// Returns false on failure.
    #[inline]
    pub fn drop_piece(&mut self, mut index: u8, team: Connect4Team) -> bool {
        if self.try_at(index) != Some(None) {
            return false;
        }

        // Make the piece "fall" if it can
        while self.try_at(index + COLS) == Some(None) {
            index += COLS;
        }

        let team = match team {
            Connect4Team::Yellow => 1,
            Connect4Team::Red => 2,
        };

        let new_entry = team * 3u128.pow(u32::from(index));
        self.0 += new_entry;

        true
    }

    /// Get whos turn it is.
    ///
    /// Does not account for ties, do that check seperately.
    #[inline]
    pub fn get_team_turn(&self) -> Connect4Team {
        let mut y_num = 0;
        let mut r_num = 0;
        for team in self.iter().filter_map(std::convert::identity) {
            match team {
                Connect4Team::Yellow => y_num += 1,
                Connect4Team::Red => r_num += 1,
            }
        }

        if r_num > y_num {
            Connect4Team::Red
        } else {
            Connect4Team::Yellow
        }
    }

    /// Utility function for testing whether 4 indexes are populated and are the same team.
    #[inline]
    fn check_indexes(self, one: u8, two: u8, three: u8, four: u8) -> Option<Connect4Team> {
        let one = self.at(one)?;
        let two = self.at(two)?;
        let three = self.at(three)?;
        let four = self.at(four)?;

        if one == two && two == three && three == four {
            return Some(one);
        }

        None
    }

    /// Get the winning team if there is one.
    pub fn get_winning_team(self) -> Option<Connect4Team> {
        // Vertical
        // For each column, do vertical check. Repeat 3 times.
        for i in 0..(COLS * 3) {
            if let Some(team) = self.check_indexes(
                i + (0 * COLS),
                i + (1 * COLS),
                i + (2 * COLS),
                i + (3 * COLS),
            ) {
                return Some(team);
            }
        }

        // Horizontal
        // For each row, do a check. Move right 3 times since COLS - 4 = 3.
        // This checks for a horizontal win in the entire row.
        for i in 0..ROWS {
            for j in 0..4 {
                if let Some(team) = self.check_indexes(i + j + 0, i + j + 1, i + j + 2, i + j + 3) {
                    return Some(team);
                }
            }
        }

        // Diagonal
        // For each row (minus the bottom half where no wins exist), do a check.
        // Move to the right 3 since a diagonal may exist up to 3 tiles away.
        for row in 0..(ROWS - 3) {
            for i in 0..4 {
                if let Some(team) = self.check_indexes(
                    i + 0 + (0 * COLS) + (row * COLS),
                    i + 1 + (1 * COLS) + (row * COLS),
                    i + 2 + (2 * COLS) + (row * COLS),
                    i + 3 + (3 * COLS) + (row * COLS),
                ) {
                    return Some(team);
                }
            }
        }

        // Anti-Diagonal
        // For each row (minus the bottom half where no wins exist), do a check.
        // Move to the right 3 since a diagonal may exist up to 3 tiles away.
        for row in 3..ROWS {
            for i in 0..4 {
                if let Some(team) = self.check_indexes(
                    i + (row * COLS) - (0 * COLS),
                    i + (row * COLS) - (1 * COLS),
                    i + (row * COLS) - (2 * COLS),
                    i + (row * COLS) - (3 * COLS),
                ) {
                    return Some(team);
                }
            }
        }

        None
    }
}

impl Default for Connect4State {
    fn default() -> Self {
        Self::new()
    }
}

/// A Connect 4 Team
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Connect4Team {
    Yellow,
    Red,
}

/// Connect 4 RuleSet
#[derive(Debug)]
pub struct Connect4RuleSet;

impl RuleSet for Connect4RuleSet {
    type State = Connect4State;
    type Team = Connect4Team;

    fn get_start_state() -> Self::State {
        Self::State::default()
    }

    fn get_team(state: &Self::State) -> Self::Team {
        state.get_team_turn()
    }

    fn get_winner(state: &Self::State) -> Option<Self::Team> {
        if let Some(team) = state.get_winning_team() {
            return Some(team);
        }

        // TODO: Anti-Diagonal

        None
    }

    fn get_child_states(state: &Self::State) -> Vec<Self::State> {
        let mut ret = Vec::with_capacity(COLS.into());

        // Assume states are valid
        for i in 0..COLS {
            let team = state.get_team_turn();
            let mut state = *state;
            if state.drop_piece(i, team) {
                ret.push(state);
            }
        }

        ret
    }

    fn score_winner(winner: &Self::Team, score: &mut i8) {
        match winner {
            Connect4Team::Yellow => *score = 1,
            Connect4Team::Red => *score = -1,
        }
    }

    fn score_state(state: &Self::State, scores: &[i8]) -> i8 {
        if Self::get_team(state) == Connect4Team::Yellow {
            scores.iter().copied().max().unwrap_or(0)
        } else {
            scores.iter().copied().min().unwrap_or(0)
        }
    }

    fn choose_best_state<'a>(
        state: &'a Self::State,
        state_score: i8,
        best_state: &mut &'a Self::State,
        best_state_score: i8,
        team: &Self::Team,
    ) {
        if (team == &Connect4Team::Yellow && best_state_score < state_score)
            || (team == &Connect4Team::Red && best_state_score > state_score)
        {
            *best_state = state;
        }
    }
}

/// An `Iterator` over a Connect 4 board.
///
/// The first result is the top left, while the last is the bottom right.
#[derive(Debug)]
pub struct Connect4Iter {
    state: u128,
    count: usize,
}

impl Connect4Iter {
    /// Make a new [`Connect4Iter`].
    ///
    /// # Note
    /// This can accept invalid states, such as states longer than [`SLOTS`] items,
    /// but it will stop yielding items afer [`SLOTS`] items have been yielded.
    pub fn new(state: u128) -> Self {
        Self { state, count: 0 }
    }
}

impl Iterator for Connect4Iter {
    type Item = Option<Connect4Team>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == usize::from(SLOTS) {
            return None;
        }

        self.count += 1;
        let ret = match self.state % 3 {
            0 => None,
            1 => Some(Connect4Team::Yellow),
            2 => Some(Connect4Team::Red),
            item => unreachable!("unexpected connect 4 team: {}", item),
        };
        self.state /= 3;

        Some(ret)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        compile_minimax_map,
        MiniMaxAi,
    };
    use std::time::Instant;

    #[test]
    fn it_works() {
        let start = Instant::now();
        let map = compile_minimax_map::<Connect4RuleSet>();
        dbg!(Instant::now() - start);

        dbg!(map.len());

        let ai: MiniMaxAi<Connect4RuleSet> = MiniMaxAi::new(map);

        dbg!(ai.get_node(&Connect4State(0)));
    }
}
