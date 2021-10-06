use crate::{
    Team,
    NUM_TILES,
};

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

/// The win type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WinType {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

/// Winner Info
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WinnerInfo {
    /// The winning team
    pub team: Team,

    /// The tile_indexes that are part of the win.
    ///
    /// Sorted from least to greatest.
    pub tile_indexes: [u8; 3],

    /// The win type
    pub win_type: WinType,
}

impl WinnerInfo {
    fn new(team: Team, i0: u8, i1: u8, i2: u8, win_type: WinType) -> Self {
        Self {
            team,
            tile_indexes: [i0, i1, i2],
            win_type,
        }
    }

    /// Get the least tile index
    pub fn start_tile_index(&self) -> u8 {
        self.tile_indexes[0]
    }

    /// Get the highest tile index
    pub fn end_tile_index(&self) -> u8 {
        self.tile_indexes[2]
    }
}

/// A Tic Tac Toe board
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

        if num_x > num_o {
            Team::O
        } else {
            Team::X
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

    /// Get the winner info, if there is a winner
    ///
    /// This is much slower than [`Self::get_winner`].
    pub fn get_winner_info(self) -> Option<WinnerInfo> {
        let winner = self.get_winner()?;

        let state = match winner {
            Team::X => self.x_state,
            Team::O => self.o_state,
        };

        // Vertical 1
        if (state & VERTICAL_WIN_1) == VERTICAL_WIN_1 {
            return Some(WinnerInfo::new(winner, 0, 3, 6, WinType::Vertical));
        }

        // Vertical 2
        if (state & VERTICAL_WIN_2) == VERTICAL_WIN_2 {
            return Some(WinnerInfo::new(winner, 0, 3, 6, WinType::Vertical));
        }

        // Vertical 3
        if (state & VERTICAL_WIN_3) == VERTICAL_WIN_3 {
            return Some(WinnerInfo::new(winner, 1, 4, 7, WinType::Vertical));
        }

        // Horizontal 1
        if (state & HORIZONTAL_WIN_1) == HORIZONTAL_WIN_1 {
            return Some(WinnerInfo::new(winner, 0, 1, 2, WinType::Horizontal));
        }

        // Horizontal 2
        if (state & HORIZONTAL_WIN_2) == HORIZONTAL_WIN_2 {
            return Some(WinnerInfo::new(winner, 3, 4, 5, WinType::Horizontal));
        }

        // Horizontal 3
        if (state & HORIZONTAL_WIN_3) == HORIZONTAL_WIN_3 {
            return Some(WinnerInfo::new(winner, 6, 7, 8, WinType::Horizontal));
        }

        // Diagonal
        if (state & DIAGONAL_WIN) == DIAGONAL_WIN {
            return Some(WinnerInfo::new(winner, 0, 4, 8, WinType::Diagonal));
        }

        // Anti Diagonal
        if (state & ANTI_DIAGONAL_WIN) == ANTI_DIAGONAL_WIN {
            return Some(WinnerInfo::new(winner, 2, 4, 6, WinType::AntiDiagonal));
        }

        None
    }

    /// Set the tile at the index.
    ///
    /// # Panics
    /// Panics if the index >= 9.
    #[must_use]
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
    pub fn iter_children(self) -> ChildrenIter {
        ChildrenIter::new(self)
    }

    /// Get an iterator over the tiles.
    ///
    /// The iterator starts at 0 at the top left and ends at 8 at the bottom right.
    ///
    /// # Returns
    /// Returns a tuple pair, where the first element is the index and the second is the tile value.
    pub fn iter(self) -> impl Iterator<Item = (u8, Option<Team>)> {
        let mut index = 0;
        std::iter::from_fn(move || {
            if index >= NUM_TILES {
                return None;
            }

            let index_mask = 1 << index;

            let ret = if (self.x_state & index_mask) != 0 {
                Some(Team::X)
            } else if (self.o_state & index_mask) != 0 {
                Some(Team::O)
            } else {
                None
            };

            let ret = Some((index, ret));
            index += 1;
            ret
        })
    }

    /// Encode this board as a [`u16`].
    pub fn encode_u16(self) -> u16 {
        let mut ret = 0;
        for i in (NUM_TILES - 1)..=0 {
            let tile = self.get(i);

            ret *= 3;
            ret += match tile {
                None => 0,
                Some(Team::X) => 1,
                Some(Team::O) => 2,
            };
        }
        ret
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

            let index_mask = 1 << self.index;
            let tile_does_not_exist = ((self.board.x_state | self.board.o_state) & index_mask) == 0;

            if tile_does_not_exist {
                let board = self.board.set(self.index, Some(self.turn));
                let item = Some((self.index, board));
                self.index += 1;
                return item;
            }
            self.index += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(usize::from(NUM_TILES)))
    }
}

impl std::iter::FusedIterator for ChildrenIter {}
