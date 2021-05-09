use crate::RuleSet;
use std::convert::TryInto;

/// The win type
#[derive(Debug, Copy, Clone)]
pub enum WinType {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

/// Winner Info
#[derive(Debug, Copy, Clone)]
pub struct WinnerInfo {
    /// The winning team
    pub team: TicTacToeTeam,

    /// The tile_indexes that are part of the win.
    ///
    /// Sorted from least to greatest.
    pub tile_indexes: [u8; 3],

    /// The win type
    pub win_type: WinType,
}

impl WinnerInfo {
    /// Get the least tile index
    pub fn start_tile_index(&self) -> u8 {
        self.tile_indexes[0]
    }

    /// Get the highest tile index
    pub fn end_tile_index(&self) -> u8 {
        self.tile_indexes[2]
    }
}
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
        for team in self.iter().flatten() {
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
    fn check_indexes(self, one: u8, two: u8, three: u8, win_type: WinType) -> Option<WinnerInfo> {
        let team = self.at(one)?;
        if self.at(one) == self.at(two) && self.at(two) == self.at(three) {
            return Some(WinnerInfo {
                team,
                win_type,
                tile_indexes: [one, two, three],
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
}

impl Default for TicTacToeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tic-Tac-Toe RuleSet
pub struct TicTacToeRuleSet;

impl RuleSet for TicTacToeRuleSet {
    type State = TicTacToeState;
    type Team = TicTacToeTeam;

    fn get_start_state() -> Self::State {
        Self::State::default()
    }

    fn get_team(state: &Self::State) -> Self::Team {
        state.get_team_turn()
    }

    fn get_winner(state: &Self::State) -> Option<Self::Team> {
        state.get_winning_team()
    }

    fn get_child_states(state: &Self::State) -> Vec<Self::State> {
        state.get_child_states()
    }

    fn score_winner(winner: &Self::Team, score: &mut i8) {
        match winner {
            TicTacToeTeam::X => *score = 1,
            TicTacToeTeam::O => *score = -1,
        }
    }

    fn score_state(state: &Self::State, scores: &[i8]) -> i8 {
        if Self::get_team(state) == TicTacToeTeam::X {
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
        if (team == &TicTacToeTeam::X && best_state_score < state_score)
            || (team == &TicTacToeTeam::O && best_state_score > state_score)
        {
            *best_state = state;
        }
    }
}

/// An `Iterator` over a Tic-Tac-Toe board.
///
/// Using this is the easiest way to interact with tic-tac-toe states.
///
/// Yields results like this:
///
/// ```ignore
/// *===*===*===*
/// | 0 | 1 | 2 |
/// *===*===*===*
/// | 3 | 4 | 5 |
/// *===*===*===*
/// | 6 | 7 | 8 |
/// *===*===*===*
/// ```
#[derive(Debug)]
pub struct TicTacToeIter {
    state: u16,
    count: usize,
}

impl TicTacToeIter {
    /// Make a new [`TicTacToeIter`].
    ///
    /// # Note
    /// This can accept invalid states, such as states longer than 9 items,
    /// but it will stop yielding items afer 9 items have been yielded.
    ///
    pub fn new(state: u16) -> Self {
        Self { state, count: 0 }
    }
}

impl Iterator for TicTacToeIter {
    type Item = Option<TicTacToeTeam>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 9 {
            return None;
        }

        self.count += 1;
        let ret = match self.state % 3 {
            0 => None,
            1 => Some(TicTacToeTeam::X),
            2 => Some(TicTacToeTeam::O),
            item => unreachable!("unexpected tic tac toe team: {}", item),
        };
        self.state /= 3;

        Some(ret)
    }
}

/// The teams of Tic-Tac-Toe.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TicTacToeTeam {
    X,
    O,
}

/// Failed to parse a [`TicTacToeTeam`] from a [`char`].
///
#[derive(Debug, Clone)]
pub struct InvalidCharError(pub char);

impl std::fmt::Display for InvalidCharError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a valid Tic-Tac-Toe team", self.0)
    }
}

impl std::error::Error for InvalidCharError {}

/// Failed to parse a [`TicTacToeTeam`] from a [`str`].
///
#[derive(Debug, Clone)]
pub enum InvalidStrError {
    /// The string is the wrong length. It must contain exactly one ascii char.
    ///
    /// The length is in bytes.
    /// For another metric, just calculate it yourself on failure.
    ///
    InvalidLength(usize),

    /// The char is not valid.
    ///
    InvalidChar(InvalidCharError),
}

impl From<InvalidCharError> for InvalidStrError {
    fn from(e: InvalidCharError) -> Self {
        Self::InvalidChar(e)
    }
}

impl std::fmt::Display for InvalidStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(len) => write!(
                f,
                "a Tic-Tac-Toe team cannot be made from inputs of length {}",
                len
            ),
            Self::InvalidChar(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for InvalidStrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::InvalidChar(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

impl TicTacToeTeam {
    /// Invert the teams
    ///
    pub fn inverse(self) -> Self {
        match self {
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }

    /// Try to parse a [`TicTacToeTeam`] from a [`char`].
    ///
    pub fn from_char(c: char) -> Result<Self, InvalidCharError> {
        match c {
            'x' | 'X' => Ok(Self::X),
            'o' | 'O' => Ok(Self::O),
            c => Err(InvalidCharError(c)),
        }
    }
}

impl std::str::FromStr for TicTacToeTeam {
    type Err = InvalidStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // This may be in bytes but this only works if the first char is ascii.
        // Therefore, this is fine.
        if s.len() != 1 {
            return Err(InvalidStrError::InvalidLength(s.len()));
        }

        Ok(TicTacToeTeam::from_char(
            s.chars().next().expect("missing char"),
        )?)
    }
}
