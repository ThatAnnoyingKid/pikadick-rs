mod state;

pub use self::state::TicTacToeState;
use crate::RuleSet;

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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TicTacToeTeam {
    X,
    O,
}

/// Failed to parse a [`TicTacToeTeam`] from a [`char`].
#[derive(Debug, Clone)]
pub struct InvalidCharError(pub char);

impl std::fmt::Display for InvalidCharError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a valid Tic-Tac-Toe team", self.0)
    }
}

impl std::error::Error for InvalidCharError {}

/// Failed to parse a [`TicTacToeTeam`] from a [`str`].
#[derive(Debug, Clone)]
pub enum InvalidStrError {
    /// The string is the wrong length. It must contain exactly one ascii char.
    ///
    /// The length is in bytes.
    /// For another metric, just calculate it yourself on failure.
    InvalidLength(usize),

    /// The char is not valid.
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
    pub fn inverse(self) -> Self {
        match self {
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }

    /// Try to parse a [`TicTacToeTeam`] from a [`char`].
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        compile_minimax_map,
        MiniMaxAi,
    };

    #[test]
    fn it_works() {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        dbg!(map.len());

        let ai: MiniMaxAi<TicTacToeRuleSet> = MiniMaxAi::new(map);

        dbg!(ai.get_move(&TicTacToeState::default(), &TicTacToeTeam::X));
    }

    #[test]
    fn delayed_win() {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        let ai = MiniMaxAi::<TicTacToeRuleSet>::new(map);

        let delayed_win_state = 13411;

        let node = ai
            .get_node(&TicTacToeState::from_u16(delayed_win_state))
            .expect("missing node");
        assert_eq!(node.score, 1);
    }
}
