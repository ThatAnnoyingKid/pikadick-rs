/// Failed to parse a [`Team`] from a [`char`].
#[derive(Debug, Clone)]
pub struct InvalidCharError(pub char);

impl std::fmt::Display for InvalidCharError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a valid Tic-Tac-Toe team", self.0)
    }
}

impl std::error::Error for InvalidCharError {}

/// Failed to parse a [`Team`] from a [`str`].
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

/// A Tic Tac Toe Team
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Team {
    X,
    O,
}

impl Team {
    /// Invert the teams
    #[must_use]
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

impl std::str::FromStr for Team {
    type Err = InvalidStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // This may be in bytes but this only works if the first char is ascii.
        // Therefore, this is fine.
        if s.len() != 1 {
            return Err(InvalidStrError::InvalidLength(s.len()));
        }

        Ok(Self::from_char(s.chars().next().expect("missing char"))?)
    }
}
