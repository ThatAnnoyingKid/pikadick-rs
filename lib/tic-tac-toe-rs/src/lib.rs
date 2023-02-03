#![allow(clippy::uninlined_format_args)]

pub mod board;
pub mod team;

pub use self::{
    board::{
        Board,
        WinType,
        WinnerInfo,
    },
    team::Team,
};

/// The # of tic-tac-toe tiles
pub const NUM_TILES: u8 = 9;

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
