use crate::RuleSet;
use std::convert::TryInto;

/// Tic-Tac-Toe RuleSet
///
pub struct TicTacToeRuleSet;

impl RuleSet for TicTacToeRuleSet {
    /// `0xFFFF > (3 ^ 9)`, so `u16` is good.
    ///
    type State = u16;
    type Team = TicTacToeTeam;

    fn get_start_state() -> Self::State {
        0
    }

    fn get_team(state: &Self::State) -> Self::Team {
        let mut x_num = 0;
        let mut o_num = 0;
        for team in TicTacToeIter::new(*state).filter_map(std::convert::identity) {
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

    fn get_winner(state: &Self::State) -> Option<Self::Team> {
        let mut horizontal_iter = TicTacToeIter::new(*state);
        for _ in 0..3 {
            let square0 = horizontal_iter.next().expect("missing square 0");
            let square1 = horizontal_iter.next().expect("missing square 1");
            let square2 = horizontal_iter.next().expect("missing square 2");

            if let Some(team) = square0 {
                if square0 == square1 && square1 == square2 {
                    return Some(team);
                }
            }
        }

        // 0, 3, 6
        // 1, 4, 7
        // 2, 5, 8
        //
        for i in 0..3 {
            let mut vertical_iter = TicTacToeIter::new(*state);
            let square0 = vertical_iter.nth(i).expect("missing square 0");
            let square1 = vertical_iter.nth(3 - 1).expect("missing square 1");
            let square2 = vertical_iter.nth(3 - 1).expect("missing square 2");

            if let Some(team) = square0 {
                if square0 == square1 && square1 == square2 {
                    return Some(team);
                }
            }
        }

        // 0, 4, 8
        // 2, 4, 6
        //
        // i * 2 = 0, 4 - (i * 2) + (i * 2) = 4, 2 * (4 - (i * 2)) + (i * 2) = 8
        // i * 2 = 2, 4 - (i * 2) + (i * 2) = 4, 2 * (4 - (i * 2)) + (i * 2) = 6
        //
        for i in 0..2 {
            let mut diagonal_iter = TicTacToeIter::new(*state);
            let square0 = diagonal_iter.nth(i * 2).expect("missing square 0");
            let square1 = diagonal_iter
                .nth(4 - (i * 2) - 1)
                .expect("missing square 1");
            let square2 = diagonal_iter
                .nth(4 - (i * 2) - 1)
                .expect("missing square 2");

            if let Some(team) = square0 {
                if square0 == square1 && square1 == square2 {
                    return Some(team);
                }
            }
        }

        None
    }

    fn get_child_states(state: &Self::State) -> Vec<Self::State> {
        let team = Self::get_team(state);

        let mut states = Vec::with_capacity(9);

        for (i, tile_team) in TicTacToeIter::new(*state).enumerate() {
            if tile_team.is_none() {
                let team = match team {
                    TicTacToeTeam::X => 1,
                    TicTacToeTeam::O => 2,
                };
                let i_u32 = i.try_into().expect("could not fit index in a `u32`");
                let new_entry = team * 3u16.pow(i_u32);
                let new_state = state + new_entry;

                states.push(new_state);
            }
        }

        states
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
///
/// Tic-Tac-Toe states are stored like this in a `u16`:
/// `t * 3.pow(i)` where
/// i is the index of the tic-tac-toe board.
/// t is 0 <= t < 3.
/// t == 0 is empty.
/// t == 1 is X.
/// t == 2 is O.
///
///
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
