pub mod model;

use crate::database::model::{
    TicTacToeGame,
    TicTacToePlayer,
};
use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::*;
use std::{
    borrow::Cow,
    path::Path,
    sync::Arc,
};

const SETUP_TABLES_SQL: &str = include_str!("../sql/setup_tables.sql");
const GET_STORE_SQL: &str = "SELECT key_value FROM kv_store WHERE key_prefix = ? AND key_name = ?;";
const PUT_STORE_SQL: &str =
    "INSERT OR REPLACE INTO kv_store (key_prefix, key_name, key_value) VALUES (?, ?, ?);";

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

/// The database
#[derive(Clone, Debug)]
pub struct Database {
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl Database {
    //// Make a new [`Database`].
    pub async fn new(path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
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

    /// Disables or enables a command.
    ///
    /// # Returns
    /// Returns the old setting
    pub async fn set_disabled_command(
        &self,
        id: GuildId,
        cmd: &str,
        disable: bool,
    ) -> anyhow::Result<bool> {
        const SELECT_QUERY: &str =
            "SELECT disabled from disabled_commands WHERE guild_id = ? AND name = ?;";
        const UPDATE_QUERY: &str =
            "INSERT OR REPLACE INTO disabled_commands (guild_id, name, disabled) VALUES (?, ?, ?);";

        let cmd = cmd.to_string();
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_value = txn
                .prepare_cached(SELECT_QUERY)?
                .query_row(params![i64::from(id), cmd], |row| row.get(0))
                .optional()?
                .unwrap_or(false);

            txn.prepare_cached(UPDATE_QUERY)?
                .execute(params![i64::from(id), cmd, disable])?;
            txn.commit()
                .context("failed to update disabled command")
                .map(|_| old_value)
        })
        .await?
    }

    /// Check if a command is disabled
    pub async fn is_command_disabled(&self, id: GuildId, name: &str) -> anyhow::Result<bool> {
        let name = name.to_string();
        self.access_db(move |db| {
            db.prepare_cached(
                "SELECT disabled FROM disabled_commands WHERE guild_id = ? AND name = ?",
            )?
            .query_row(params![i64::from(id), name], |row| row.get(0))
            .optional()
            .context("failed to access db")
            .map(|row| row.unwrap_or(false))
        })
        .await?
    }

    /// Get a key from the store
    pub async fn store_get<P, K, V>(&self, prefix: P, key: K) -> anyhow::Result<Option<V>>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        let maybe_bytes: Option<Vec<u8>> = self
            .access_db(move |db| {
                db.prepare_cached(GET_STORE_SQL)?
                    .query_row([prefix, key], |row| row.get(0))
                    .optional()
                    .context("failed to get value")
            })
            .await??;

        match maybe_bytes {
            Some(bytes) => Ok(Some(
                bincode::deserialize(&bytes).context("failed to decode value")?,
            )),
            None => Ok(None),
        }
    }

    /// Put a key in the store
    pub async fn store_put<P, K, V>(&self, prefix: P, key: K, value: V) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();
        let value = bincode::serialize(&value).context("failed to serialize value")?;

        self.access_db(move |db| {
            let txn = db.transaction()?;
            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }

    /// Get and Put a key in the store in one action, ensuring the key is not changed between the commands.
    pub async fn store_update<P, K, V, U>(
        &self,
        prefix: P,
        key: K,
        update_func: U,
    ) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize + serde::de::DeserializeOwned,
        U: FnOnce(Option<V>) -> V + Send + 'static,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;

            let maybe_value = txn
                .prepare_cached(GET_STORE_SQL)?
                .query_row(params![prefix, key], |row| row.get(0))
                .optional()
                .context("failed to get value")?
                .map(|bytes: Vec<u8>| {
                    bincode::deserialize(&bytes).context("failed to decode value")
                })
                .transpose()?;
            let value = update_func(maybe_value);
            let value = bincode::serialize(&value).context("failed to serialize value")?;

            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }

    /// Enable or disable reddit embeds.
    ///
    /// # Returns
    /// Returns the old value
    pub async fn set_reddit_embed_enabled(
        &self,
        guild_id: GuildId,
        enabled: bool,
    ) -> anyhow::Result<bool> {
        const SELECT_QUERY: &str =
            "SELECT enabled FROM reddit_embed_guild_settings WHERE guild_id = ?";
        const INSERT_QUERY: &str =
            "INSERT OR REPLACE INTO reddit_embed_guild_settings (guild_id, enabled) VALUES (?, ?);";
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_data = txn
                .prepare_cached(SELECT_QUERY)?
                .query_row([i64::from(guild_id)], |row| row.get(0))
                .optional()?
                .unwrap_or(false);
            txn.prepare_cached(INSERT_QUERY)?
                .execute(params![i64::from(guild_id), enabled])?;

            txn.commit()
                .context("failed to set reddit embed")
                .map(|_| old_data)
        })
        .await?
    }

    /// Get the reddit embed setting.
    pub async fn get_reddit_embed_enabled(&self, guild_id: GuildId) -> anyhow::Result<bool> {
        const SELECT_QUERY: &str =
            "SELECT enabled FROM reddit_embed_guild_settings WHERE guild_id = ?";
        self.access_db(move |db| {
            db.prepare_cached(SELECT_QUERY)?
                .query_row([i64::from(guild_id)], |row| row.get(0))
                .optional()
                .context("failed to read database")
                .map(|v| v.unwrap_or(false))
        })
        .await?
    }

    /// Create a new tic-tac-toe game
    pub async fn create_tic_tac_toe_game(
        &self,
        x_player: TicTacToePlayer,
        o_player: TicTacToePlayer,
    ) -> Result<(i64, TicTacToeGame), TicTacToeCreateGameError> {
        let query = "INSERT INTO tic_tac_toe_games (board, x_player, o_player) VALUES (?, ?, ?) RETURNING id";
        self.access_db(move |db| {
            let txn = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .context("failed to create transaction")
                .map_err(TicTacToeCreateGameError::Database)?;

            let mut game = TicTacToeGame::new(x_player, o_player);

            // TODO: Iteratively perform AI steps
            if x_player.is_computer() {
                let (_score, index) = tic_tac_toe::minimax(game.board, tic_tac_toe::NUM_TILES);
                game.board = game.board.set(index, Some(tic_tac_toe::Team::X));
            }

            let board = game.board.encode_u16();
            let x_player: Cow<'static, _> = game.x_player.into();
            let o_player: Cow<'static, _> = game.x_player.into();
            let game_id: i64 = txn
                .prepare_cached(query)
                .context("failed to prepare query")
                .map_err(TicTacToeCreateGameError::Database)?
                .query_row(params![board, x_player, o_player], |row| row.get(0))
                .context("failed to execute query")
                .map_err(TicTacToeCreateGameError::Database)?;

            txn.commit()
                .context("failed to commit")
                .map_err(TicTacToeCreateGameError::Database)?;

            Ok((game_id, game))
        })
        .await
        .context("database access failed to join")
        .map_err(TicTacToeCreateGameError::Database)?
    }

    /// Try to make a tic-tac-toe move
    pub async fn try_tic_tac_toe_move(
        &self,
        id: i64,
        author_id: UserId,
        move_index: u8,
    ) -> Result<TicTacToeTryMoveResponse, TicTacToeTryMoveError> {
        self.access_db(move |db| {
            let txn = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .context("failed to create transaction")
                .map_err(TicTacToeTryMoveError::Database)?;
            let mut game = txn
                .prepare_cached(
                    "SELECT board, x_player, o_player FROM tic_tac_toe_games WHERE id = ?;",
                )
                .context("failed to prepare query")
                .map_err(TicTacToeTryMoveError::Database)?
                .query_row([id], |row| {
                    let board: u16 = row.get(0)?;
                    let x_player: String = row.get(1)?;
                    let o_player: String = row.get(2)?;

                    let maybe_x_player = x_player
                        .parse::<TicTacToePlayer>()
                        .context("failed to parse x player");
                    let maybe_o_player = o_player
                        .parse::<TicTacToePlayer>()
                        .context("failed to parse o player");

                    Ok(maybe_x_player
                        .and_then(|x_player| Ok((x_player, maybe_o_player?)))
                        .map(|(x_player, o_player)| TicTacToeGame {
                            board: tic_tac_toe::Board::decode_u16(board),
                            x_player,
                            o_player,
                        }))
                })
                .context("failed to query database")
                .map_err(TicTacToeTryMoveError::Database)?
                .map_err(TicTacToeTryMoveError::Database)?;

            let player_turn = game.get_player_turn();
            if TicTacToePlayer::User(author_id) != player_turn {
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

                txn.prepare_cached("DELETE FROM tic_tac_toe_games WHERE id = ?;")
                    .context("failed to prepare query")
                    .map_err(TicTacToeTryMoveError::Database)?
                    .execute([id])
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
                txn.prepare_cached("DELETE FROM tic_tac_toe_games WHERE id = ?;")
                    .context("failed to prepare query")
                    .map_err(TicTacToeTryMoveError::Database)?
                    .execute([id])
                    .context("failed to delete game")
                    .map_err(TicTacToeTryMoveError::Database)?;

                txn.commit()
                    .context("failed to commit")
                    .map_err(TicTacToeTryMoveError::Database)?;

                return Ok(TicTacToeTryMoveResponse::Tie { game });
            }

            let board = game.board.encode_u16();
            txn.prepare_cached("UPDATE tic_tac_toe_games SET board = ? WHERE id = ?;")
                .context("failed to prepare query")
                .map_err(TicTacToeTryMoveError::Database)?
                .execute(params![board, id])
                .context("failed to delete game")
                .map_err(TicTacToeTryMoveError::Database)?;

            let opponent = game.get_player_turn();
            if opponent == TicTacToePlayer::Computer {
                let (_score, index) = tic_tac_toe::minimax(game.board, tic_tac_toe::NUM_TILES);
                game.board = game.board.set(index, Some(team_turn.inverse()));

                if let Some(winner_team) = game.board.get_winner() {
                    let winner_player = game.get_player(winner_team);
                    let loser_player = game.get_player(winner_team.inverse());

                    txn.prepare_cached("DELETE FROM tic_tac_toe_games WHERE id = ?;")
                        .context("failed to prepare query")
                        .map_err(TicTacToeTryMoveError::Database)?
                        .execute([id])
                        .context("failed to delete game")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    return Ok(TicTacToeTryMoveResponse::Winner {
                        game,
                        winner: winner_player,
                        loser: loser_player,
                    });
                }

                if game.board.is_draw() {
                    txn.prepare_cached("DELETE FROM tic_tac_toe_games WHERE id = ?;")
                        .context("failed to prepare query")
                        .map_err(TicTacToeTryMoveError::Database)?
                        .execute([id])
                        .context("failed to delete game")
                        .map_err(TicTacToeTryMoveError::Database)?;

                    return Ok(TicTacToeTryMoveResponse::Tie { game });
                }
            }

            let board = game.board.encode_u16();
            txn.prepare_cached("UPDATE tic_tac_toe_games SET board = ? WHERE id = ?;")
                .context("failed to prepare query")
                .map_err(TicTacToeTryMoveError::Database)?
                .execute(params![board, id])
                .context("failed to delete game")
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
}
