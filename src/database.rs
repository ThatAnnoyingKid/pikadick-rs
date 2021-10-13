mod disabled_commands;
mod kv_store;
pub mod model;
mod reddit_embed;

use crate::database::model::{
    MaybeGuildString,
    TicTacToeGame,
    TicTacToePlayer,
};
use anyhow::Context;
use once_cell::sync::Lazy;
use rusqlite::{
    named_params,
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::*;
use std::{
    os::raw::c_int,
    path::Path,
    sync::Arc,
};
use tracing::warn;

// Setup
const SETUP_TABLES_SQL: &str = include_str!("../sql/setup_tables.sql");

// Tic-Tac-Toe
const DELETE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../sql/delete_tic_tac_toe_game.sql");
const UPDATE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../sql/update_tic_tac_toe_game.sql");
const CREATE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../sql/create_tic_tac_toe_game.sql");
const GET_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../sql/get_tic_tac_toe_game.sql");
const CHECK_IN_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../sql/check_in_tic_tac_toe_game.sql");

static LOGGER_INIT: Lazy<Result<(), Arc<rusqlite::Error>>> = Lazy::new(|| {
    // Safety:
    // 1. `sqlite_logger_func` is threadsafe.
    // 2. This is called only once.
    // 3. This is called before any sqlite functions are used
    // 4. sqlite functions cannot be used until the logger initializes.
    unsafe { rusqlite::trace::config_log(Some(sqlite_logger_func)).map_err(Arc::new) }
});

/// Error that may occur while creating a tic-tac-toe game
#[derive(Debug, thiserror::Error)]
pub enum TicTacToeCreateGameError {
    /// The author is in a game
    #[error("the author is in a game")]
    AuthorInGame,

    /// The opponent is in a game
    #[error("the opponent is in a game")]
    OpponentInGame,

    /// Error accessing the database
    #[error("database error")]
    Database(#[source] anyhow::Error),
}

/// Error that may occur while performing a tic-tac-toe move
#[derive(Debug, thiserror::Error)]
pub enum TicTacToeTryMoveError {
    /// The user is not in a game
    #[error("not in a game")]
    NotInAGame,

    /// It is not the user's turn
    #[error("not the user's turn to move")]
    InvalidTurn,

    /// The move is invalid
    #[error("the move is not valid")]
    InvalidMove,

    /// Error accessing the database
    #[error("database error")]
    Database(#[source] anyhow::Error),
}

/// The response for making a tic-tac-toe move
#[derive(Debug, Copy, Clone)]
pub enum TicTacToeTryMoveResponse {
    /// There was a winner
    Winner {
        game: TicTacToeGame,
        winner: TicTacToePlayer,
        loser: TicTacToePlayer,
    },
    /// There was a tie
    Tie { game: TicTacToeGame },
    /// The next turn executed
    NextTurn { game: TicTacToeGame },
}

fn delete_tic_tac_toe_game(txn: &rusqlite::Transaction<'_>, id: i64) -> rusqlite::Result<()> {
    txn.prepare_cached(DELETE_TIC_TAC_TOE_GAME_SQL)?
        .execute([id])?;
    Ok(())
}

fn update_tic_tac_toe_game(
    txn: &rusqlite::Transaction<'_>,
    id: i64,
    board: tic_tac_toe::Board,
) -> rusqlite::Result<()> {
    txn.prepare_cached(UPDATE_TIC_TAC_TOE_GAME_SQL)?
        .execute(params![board.encode_u16(), id])?;
    Ok(())
}

fn get_tic_tac_toe_game(
    txn: &rusqlite::Transaction<'_>,
    guild_id: MaybeGuildString,
    user_id: TicTacToePlayer,
) -> rusqlite::Result<Option<(i64, TicTacToeGame)>> {
    txn.prepare_cached(GET_TIC_TAC_TOE_GAME_SQL)?
        .query_row(
            named_params! {
                ":guild_id": guild_id,
                ":user_id": user_id
            },
            |row| {
                Ok((
                    row.get(0)?,
                    TicTacToeGame {
                        board: tic_tac_toe::Board::decode_u16(row.get(1)?),
                        x_player: row.get(2)?,
                        o_player: row.get(3)?,
                    },
                ))
            },
        )
        .optional()
}

fn sqlite_logger_func(error_code: c_int, msg: &str) {
    warn!("sqlite error code ({}): {}", error_code, msg);
}

/// The database
#[derive(Clone, Debug)]
pub struct Database {
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl Database {
    //// Make a new [`Database`].
    pub async fn new(path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
        LOGGER_INIT
            .clone()
            .context("failed to init sqlite logger")?;

        let mut flags = rusqlite::OpenFlags::default();
        if !create_if_missing {
            flags.remove(rusqlite::OpenFlags::SQLITE_OPEN_CREATE)
        }
        let path = path.to_owned();
        let db: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let db = rusqlite::Connection::open_with_flags(path, flags)
                .context("failed to open database")?;
            db.execute_batch(SETUP_TABLES_SQL)
                .context("failed to setup database")?;
            Ok(Arc::new(parking_lot::Mutex::new(db)))
        })
        .await?;
        let db = db?;

        Ok(Database { db })
    }

    /// Access the db on the tokio threadpool
    async fn access_db<F, R>(&self, func: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut rusqlite::Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        let db = self.db.clone();
        Ok(tokio::task::spawn_blocking(move || func(&mut db.lock())).await?)
    }

    /// Create a new tic-tac-toe game
    pub async fn create_tic_tac_toe_game(
        &self,
        guild_id: MaybeGuildString,
        author: TicTacToePlayer,
        author_team: tic_tac_toe::Team,
        opponent: TicTacToePlayer,
    ) -> Result<TicTacToeGame, TicTacToeCreateGameError> {
        let (x_player, o_player) = if author_team == tic_tac_toe::Team::X {
            (author, opponent)
        } else {
            (opponent, author)
        };

        self.access_db(move |db| {
            let txn = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .context("failed to create transaction")
                .map_err(TicTacToeCreateGameError::Database)?;

            let check_in_game_result: Option<(TicTacToePlayer, TicTacToePlayer)> = txn
                .prepare_cached(CHECK_IN_TIC_TAC_TOE_GAME_SQL)
                .context("failed to prepare query")
                .map_err(TicTacToeCreateGameError::Database)?
                .query_row(
                    named_params! {
                        ":guild_id": guild_id,
                        ":author": author,
                        ":opponent": opponent,
                    },
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .context("failed to query if in game")
                .map_err(TicTacToeCreateGameError::Database)?;

            if let Some((maybe_x_player_in_game, maybe_o_player_in_game)) = check_in_game_result {
                if maybe_x_player_in_game == author || maybe_o_player_in_game == author {
                    return Err(TicTacToeCreateGameError::AuthorInGame);
                }

                if maybe_x_player_in_game == opponent || maybe_o_player_in_game == opponent {
                    return Err(TicTacToeCreateGameError::OpponentInGame);
                }
            }

            let mut game = TicTacToeGame::new(x_player, o_player);

            // TODO: Iteratively perform AI steps?
            if x_player.is_computer() {
                let (_score, index) = tic_tac_toe::minimax(game.board, tic_tac_toe::NUM_TILES);
                game.board = game.board.set(index, Some(tic_tac_toe::Team::X));
            }

            let board = game.board.encode_u16();
            txn.prepare_cached(CREATE_TIC_TAC_TOE_GAME_SQL)
                .context("failed to prepare query")
                .map_err(TicTacToeCreateGameError::Database)?
                .execute(params![board, x_player, o_player, guild_id])
                .context("failed to create game in database")
                .map_err(TicTacToeCreateGameError::Database)?;

            txn.commit()
                .context("failed to commit")
                .map_err(TicTacToeCreateGameError::Database)?;

            Ok(game)
        })
        .await
        .context("database access failed to join")
        .map_err(TicTacToeCreateGameError::Database)?
    }

    /// Try to make a tic-tac-toe move
    pub async fn try_tic_tac_toe_move(
        &self,
        guild_id: MaybeGuildString,
        player: TicTacToePlayer,
        move_index: u8,
    ) -> Result<TicTacToeTryMoveResponse, TicTacToeTryMoveError> {
        self.access_db(move |db| {
            let txn = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .context("failed to create transaction")
                .map_err(TicTacToeTryMoveError::Database)?;

            let (id, mut game) = get_tic_tac_toe_game(&txn, guild_id, player)
                .context("failed to get game")
                .map_err(TicTacToeTryMoveError::Database)?
                .ok_or(TicTacToeTryMoveError::NotInAGame)?;

            let player_turn = game.get_player_turn();
            if player != player_turn {
                return Err(TicTacToeTryMoveError::InvalidTurn);
            }

            let team_turn = game.get_team_turn();
            let move_successful = game.try_move(move_index, team_turn);
            if !move_successful {
                return Err(TicTacToeTryMoveError::InvalidMove);
            }

            if let Some(winner_team) = game.board.get_winner() {
                let winner = game.get_player(winner_team);
                let loser = game.get_player(winner_team.inverse());

                delete_tic_tac_toe_game(&txn, id)
                    .context("failed to delete game")
                    .map_err(TicTacToeTryMoveError::Database)?;

                txn.commit()
                    .context("failed to commit")
                    .map_err(TicTacToeTryMoveError::Database)?;

                return Ok(TicTacToeTryMoveResponse::Winner {
                    game,
                    winner,
                    loser,
                });
            }

            if game.board.is_draw() {
                delete_tic_tac_toe_game(&txn, id)
                    .context("failed to delete game")
                    .map_err(TicTacToeTryMoveError::Database)?;

                txn.commit()
                    .context("failed to commit")
                    .map_err(TicTacToeTryMoveError::Database)?;

                return Ok(TicTacToeTryMoveResponse::Tie { game });
            }

            update_tic_tac_toe_game(&txn, id, game.board)
                .context("failed to update game")
                .map_err(TicTacToeTryMoveError::Database)?;

            let opponent = game.get_player_turn();
            if opponent == TicTacToePlayer::Computer {
                let (_score, index) = tic_tac_toe::minimax(game.board, tic_tac_toe::NUM_TILES);
                game.board = game.board.set(index, Some(team_turn.inverse()));

                if let Some(winner_team) = game.board.get_winner() {
                    let winner_player = game.get_player(winner_team);
                    let loser_player = game.get_player(winner_team.inverse());

                    delete_tic_tac_toe_game(&txn, id)
                        .context("failed to delete game")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    txn.commit()
                        .context("failed to commit")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    return Ok(TicTacToeTryMoveResponse::Winner {
                        game,
                        winner: winner_player,
                        loser: loser_player,
                    });
                }

                if game.board.is_draw() {
                    delete_tic_tac_toe_game(&txn, id)
                        .context("failed to delete game")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    txn.commit()
                        .context("failed to commit")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    return Ok(TicTacToeTryMoveResponse::Tie { game });
                }
            }

            update_tic_tac_toe_game(&txn, id, game.board)
                .context("failed to update game")
                .map_err(TicTacToeTryMoveError::Database)?;

            txn.commit()
                .context("failed to commit")
                .map_err(TicTacToeTryMoveError::Database)?;

            Ok(TicTacToeTryMoveResponse::NextTurn { game })
        })
        .await
        .context("database access failed to join")
        .map_err(TicTacToeTryMoveError::Database)?
    }

    /// Try to get a tic-tac-toe game by guild and player
    pub async fn get_tic_tac_toe_game(
        &self,
        guild_id: MaybeGuildString,
        player: TicTacToePlayer,
    ) -> anyhow::Result<Option<TicTacToeGame>> {
        self.access_db(move |db| {
            let txn = db.transaction()?;
            let ret = get_tic_tac_toe_game(&txn, guild_id, player).context("failed to query")?;
            txn.commit()
                .context("failed to commit")
                .map(|_| ret.map(|ret| ret.1))
        })
        .await?
    }

    /// Try to delete a Tic-Tac-Toe game.
    ///
    /// # Returns
    /// Returns the game if it existed
    pub async fn delete_tic_tac_toe_game(
        &self,
        guild_id: MaybeGuildString,
        player: TicTacToePlayer,
    ) -> anyhow::Result<Option<TicTacToeGame>> {
        self.access_db(move |db| {
            let txn = db.transaction()?;
            let ret = get_tic_tac_toe_game(&txn, guild_id, player).context("failed to query")?;

            if let Some((id, _game)) = ret {
                delete_tic_tac_toe_game(&txn, id).context("failed to delete game")?;
            }

            txn.commit()
                .context("failed to commit")
                .map(|_| ret.map(|ret| ret.1))
        })
        .await?
    }
}
