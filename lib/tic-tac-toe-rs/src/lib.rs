// Allow unusual_byte_groupings as we group by 3 to visualize the board
// Vertical Wins
#[allow(clippy::unusual_byte_groupings)]
const VERTICAL_WIN_1: u16 = 0b100_100_100;
#[allow(clippy::unusual_byte_groupings)]
const VERTICAL_WIN_2: u16 = 0b010_010_010;
#[allow(clippy::unusual_byte_groupings)]
const VERTICAL_WIN_3: u16 = 0b001_001_001;

// Horizontal Wins
#[allow(clippy::unusual_byte_groupings)]
const HORIZONTAL_WIN_1: u16 = 0b111_000_000;
#[allow(clippy::unusual_byte_groupings)]
const HORIZONTAL_WIN_2: u16 = 0b000_111_000;
#[allow(clippy::unusual_byte_groupings)]
const HORIZONTAL_WIN_3: u16 = 0b000_000_111;

// Diagonal win
#[allow(clippy::unusual_byte_groupings)]
const DIAGONAL_WIN: u16 = 0b100_010_001;

// Anti-Diagonal win
#[allow(clippy::unusual_byte_groupings)]
const ANTI_DIAGONAL_WIN: u16 = 0b001_010_100;

/// The # of tic-tac-toe tiles
pub const NUM_TILES: u8 = 9;

/// A Tic Tac Toe Team
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Team {
    X,
    O,
}

/// A Tic Tac Toe board
#[derive(Debug, Copy, Clone)]
pub struct Board {
    // the bitboard
    // 9 tiles, so it cannot fit in a u8 but can fit in a u16
    x_state: u16,
    o_state: u16,
}

impl Board {
    /// Make a new [`Board`].
    pub fn new() -> Self {
        Board {
            x_state: 0,
            o_state: 0,
        }
    }

    // TODO: Maybe just add this to the board.
    /// Get the team whos turn it is.
    pub fn get_turn(self) -> Team {
        let num_x = self.x_state.count_ones();
        let num_o = self.o_state.count_ones();

        if num_x <= num_o {
            Team::X
        } else {
            Team::O
        }
    }

    /// Returns true if it is a draw.
    ///
    /// This does not check for wins.
    pub fn is_draw(self) -> bool {
        (self.x_state | self.o_state).count_ones() >= u32::from(NUM_TILES)
    }

    /// Check if the given team won.
    ///
    /// This is designed to be fast.
    pub fn has_won(self, team: Team) -> bool {
        let state = match team {
            Team::X => self.x_state,
            Team::O => self.o_state,
        };

        ((state & VERTICAL_WIN_1) == VERTICAL_WIN_1)
            || ((state & VERTICAL_WIN_2) == VERTICAL_WIN_2)
            || ((state & VERTICAL_WIN_3) == VERTICAL_WIN_3)
            || ((state & HORIZONTAL_WIN_1) == HORIZONTAL_WIN_1)
            || ((state & HORIZONTAL_WIN_2) == HORIZONTAL_WIN_2)
            || ((state & HORIZONTAL_WIN_3) == HORIZONTAL_WIN_3)
            || ((state & DIAGONAL_WIN) == DIAGONAL_WIN)
            || ((state & ANTI_DIAGONAL_WIN) == ANTI_DIAGONAL_WIN)
    }

    /// Get the winner if they exist
    pub fn get_winner(self) -> Option<Team> {
        if self.has_won(Team::X) {
            Some(Team::X)
        } else if self.has_won(Team::O) {
            Some(Team::O)
        } else {
            None
        }
    }

    /// Set the tile at the index.
    ///
    /// # Panics
    /// Panics if the index >= 9.
    pub fn set(mut self, index: u8, team: Option<Team>) -> Self {
        assert!(index < NUM_TILES);
        match team {
            Some(Team::X) => {
                self.x_state |= 1 << index;
                self.o_state &= !(1 << index);
            }
            Some(Team::O) => {
                self.x_state &= !(1 << index);
                self.o_state |= 1 << index;
            }
            None => {
                self.x_state &= !(1 << index);
                self.o_state &= !(1 << index);
            }
        }
        self
    }

    /// Get the tile at the index.
    ///
    /// # Panics
    /// Panics if the index >= 9.
    pub fn get(self, index: u8) -> Option<Team> {
        assert!(index < NUM_TILES);
        if self.x_state & (1 << index) != 0 {
            Some(Team::X)
        } else if self.o_state & (1 << index) != 0 {
            Some(Team::O)
        } else {
            None
        }
    }

    /// Get an iterator over child board states.
    ///
    /// # Returns
    /// Returns an Iterator where Items are tuples.
    /// The first item is the index of the placed tile.
    /// The second is the resulting board state.
    pub fn iter_children(self) -> impl Iterator<Item = (u8, Self)> {
        ChildrenIter::new(self)
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ChildrenIter {
    board: Board,
    turn: Team,
    index: u8,
}

impl ChildrenIter {
    fn new(board: Board) -> Self {
        let turn = board.get_turn();
        Self {
            board,
            turn,
            index: 0,
        }
    }
}

impl Iterator for ChildrenIter {
    type Item = (u8, Board);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index >= NUM_TILES {
                return None;
            }

            if self.board.get(self.index).is_none() {
                let board = self.board.set(self.index, Some(self.turn));
                let item = Some((self.index, board));
                self.index += 1;
                return item;
            }
            self.index += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(9))
    }
}

impl std::iter::FusedIterator for ChildrenIter {}

/// Run minimax on a board.
///
/// This is negamax. The returned score is relative to the current player.
///
/// # Returns
/// Returns a tuple. The first element is the score. The second is the move.
pub fn minimax(board: Board, depth: u8) -> (i8, u8) {
    let color = match board.get_turn() {
        Team::X => 1,
        Team::O => -1,
    };

    if depth == 0 {
        return (0, 0);
    }

    match board.get_winner() {
        Some(Team::X) => return (color, 0),
        Some(Team::O) => return (-color, 0),
        None => {}
    }

    if board.is_draw() {
        return (0, 0);
    }

    let mut value = i8::MIN;
    let mut best_index = 0;
    for (index, child) in board.iter_children() {
        let (new_value, _index) = minimax(child, depth - 1);
        let new_value = -new_value;

        if new_value > value {
            value = new_value;
            best_index = index;

            // If value is the max value, stop search
            if value == 1 {
                return (value, index);
            }
        }
    }

    (value, best_index)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn minimax_all() {
        let board = Board::new();
        let (score, index) = minimax(board, 9);
        assert_eq!(score, 0);
        assert_eq!(index, 0);
    }

    #[test]
    fn minimax_win_1() {
        let board = Board::new()
            .set(0, Some(Team::X))
            .set(4, Some(Team::O))
            .set(8, Some(Team::X))
            .set(2, Some(Team::O));
        let (score, index) = minimax(board, 9);
        assert_eq!(score, 1, "expected X win");
        assert_eq!(index, 6);
    }

    #[test]
    fn minimax_win_2() {
        let board = Board::new()
            .set(0, Some(Team::X))
            .set(1, Some(Team::O))
            .set(2, Some(Team::X))
            .set(4, Some(Team::O))
            .set(3, Some(Team::X));
        let (score, index) = minimax(board, 9);
        assert_eq!(score, 1, "expected O win");
        assert_eq!(index, 7);
    }
}
