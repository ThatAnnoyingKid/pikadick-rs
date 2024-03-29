use crate::database::{
    model::{
        MaybeGuildString,
        TicTacToeGame,
        TicTacToePlayer,
        TicTacToeScore,
        TicTacToeTopPlayerScore,
    },
    Database,
};
use anyhow::Context;
use rusqlite::{
    named_params,
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::*;
use tic_tac_toe::Board;

// Tic-Tac-Toe SQL
const DELETE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../../sql/delete_tic_tac_toe_game.sql");
const UPDATE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../../sql/update_tic_tac_toe_game.sql");
const CREATE_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../../sql/create_tic_tac_toe_game.sql");
const GET_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../../sql/get_tic_tac_toe_game.sql");
const CHECK_IN_TIC_TAC_TOE_GAME_SQL: &str = include_str!("../../sql/check_in_tic_tac_toe_game.sql");
const CREATE_DEFAULT_SCORE_TIC_TAC_TOE_SQL: &str =
    include_str!("../../sql/create_default_score_tic_tac_toe.sql");
const INCREMENT_TIES_SCORE_TIC_TAC_TOE_SQL: &str =
    include_str!("../../sql/increment_ties_score_tic_tac_toe.sql");
const INCREMENT_WINS_SCORE_TIC_TAC_TOE_SQL: &str =
    include_str!("../../sql/increment_wins_score_tic_tac_toe.sql");
const INCREMENT_LOSSES_SCORE_TIC_TAC_TOE_SQL: &str =
    include_str!("../../sql/increment_losses_score_tic_tac_toe.sql");
const INCREMENT_CONCEDES_SCORE_TIC_TAC_TOE_SQL: &str =
    include_str!("../../sql/increment_concedes_score_tic_tac_toe.sql");
const GET_TIC_TAC_TOE_SCORE_SQL: &str = include_str!("../../sql/get_tic_tac_toe_score.sql");
const GET_TOP_TIC_TAC_TOE_SCORES_SQL: &str =
    include_str!("../../sql/get_top_tic_tac_toe_scores.sql");

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
                        board: Board::decode_u16(row.get(1)?),
                        x_player: row.get(2)?,
                        o_player: row.get(3)?,
                    },
                ))
            },
        )
        .optional()
}

fn update_tic_tac_toe_game(
    txn: &rusqlite::Transaction<'_>,
    id: i64,
    board: Board,
) -> rusqlite::Result<()> {
    txn.prepare_cached(UPDATE_TIC_TAC_TOE_GAME_SQL)?
        .execute(params![board.encode_u16(), id])?;
    Ok(())
}

fn delete_tic_tac_toe_game(txn: &rusqlite::Transaction<'_>, id: i64) -> rusqlite::Result<()> {
    txn.prepare_cached(DELETE_TIC_TAC_TOE_GAME_SQL)?
        .execute([id])?;
    Ok(())
}

/// Try to make a user's score data
fn create_user_score_data(
    txn: &rusqlite::Transaction<'_>,
    guild_id: MaybeGuildString,
    user_id: UserId,
) -> rusqlite::Result<()> {
    txn.prepare_cached(CREATE_DEFAULT_SCORE_TIC_TAC_TOE_SQL)?
        .execute(params![guild_id, i64::from(user_id)])?;

    Ok(())
}

/// Set a tic-tac-toe game as a draw as part of a larger transaction, consuming it.
fn set_draw_tic_tac_toe_game(
    txn: rusqlite::Transaction<'_>,
    id: i64,
    guild_id: MaybeGuildString,
    game: TicTacToeGame,
) -> anyhow::Result<()> {
    delete_tic_tac_toe_game(&txn, id).context("failed to delete game")?;

    if let (TicTacToePlayer::User(x_player), TicTacToePlayer::User(o_player)) =
        (game.x_player, game.o_player)
    {
        create_user_score_data(&txn, guild_id, x_player)?;
        create_user_score_data(&txn, guild_id, o_player)?;

        txn.prepare_cached(INCREMENT_TIES_SCORE_TIC_TAC_TOE_SQL)?
            .execute(params![guild_id, i64::from(x_player), i64::from(o_player)])?;
    }

    txn.commit().context("failed to commit")?;

    Ok(())
}

/// Set a tic-tac-toe game as a win as part of a larger transaction, consuming it.
fn set_win_tic_tac_toe_game(
    txn: rusqlite::Transaction<'_>,
    id: i64,
    guild_id: MaybeGuildString,
    winner: TicTacToePlayer,
    loser: TicTacToePlayer,
) -> anyhow::Result<()> {
    delete_tic_tac_toe_game(&txn, id).context("failed to delete game")?;

    if let (TicTacToePlayer::User(winner), TicTacToePlayer::User(loser)) = (winner, loser) {
        create_user_score_data(&txn, guild_id, winner)?;
        create_user_score_data(&txn, guild_id, loser)?;

        txn.prepare_cached(INCREMENT_WINS_SCORE_TIC_TAC_TOE_SQL)?
            .execute(params![guild_id, i64::from(winner)])?;
        txn.prepare_cached(INCREMENT_LOSSES_SCORE_TIC_TAC_TOE_SQL)?
            .execute(params![guild_id, i64::from(loser)])?;
    }

    txn.commit().context("failed to commit")?;

    Ok(())
}

impl Database {
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

                set_win_tic_tac_toe_game(txn, id, guild_id, winner, loser)
                    .map_err(TicTacToeTryMoveError::Database)?;

                return Ok(TicTacToeTryMoveResponse::Winner {
                    game,
                    winner,
                    loser,
                });
            }

            if game.board.is_draw() {
                set_draw_tic_tac_toe_game(txn, id, guild_id, game)
                    .map_err(TicTacToeTryMoveError::Database)?;
                return Ok(TicTacToeTryMoveResponse::Tie { game });
            }

            let opponent = game.get_player_turn();
            if opponent == TicTacToePlayer::Computer {
                let (_score, index) = tic_tac_toe::minimax(game.board, tic_tac_toe::NUM_TILES);
                game.board = game.board.set(index, Some(team_turn.inverse()));

                if let Some(winner_team) = game.board.get_winner() {
                    let winner = game.get_player(winner_team);
                    let loser = game.get_player(winner_team.inverse());

                    set_win_tic_tac_toe_game(txn, id, guild_id, winner, loser)
                        .map_err(TicTacToeTryMoveError::Database)?;

                    return Ok(TicTacToeTryMoveResponse::Winner {
                        game,
                        winner,
                        loser,
                    });
                }

                if game.board.is_draw() {
                    set_draw_tic_tac_toe_game(txn, id, guild_id, game)
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

    /// Try to concede a Tic-Tac-Toe game.
    ///
    /// # Returns
    /// Returns the game if it existed
    pub async fn concede_tic_tac_toe_game(
        &self,
        guild_id: MaybeGuildString,
        player: UserId,
    ) -> anyhow::Result<Option<TicTacToeGame>> {
        self.access_db(move |db| {
            let txn = db.transaction()?;
            let ret =
                get_tic_tac_toe_game(&txn, guild_id, player.into()).context("failed to query")?;

            if let Some((id, game)) = ret {
                delete_tic_tac_toe_game(&txn, id).context("failed to delete game")?;

                let conceding_player = TicTacToePlayer::from(player);
                let opponent = game
                    .get_opponent(conceding_player)
                    .context("missing opponent")?;

                if let (TicTacToePlayer::User(conceding_player), TicTacToePlayer::User(opponent)) =
                    (conceding_player, opponent)
                {
                    txn.prepare_cached(INCREMENT_CONCEDES_SCORE_TIC_TAC_TOE_SQL)?
                        .execute(params![guild_id, i64::from(conceding_player)])?;

                    txn.prepare_cached(INCREMENT_WINS_SCORE_TIC_TAC_TOE_SQL)?
                        .execute(params![guild_id, i64::from(opponent)])?;
                }
            }

            txn.commit()
                .context("failed to commit")
                .map(|_| ret.map(|ret| ret.1))
        })
        .await?
    }

    /// Get the user's Tic-Tac-Toe scores
    pub async fn get_tic_tac_toe_score(
        &self,
        guild_id: MaybeGuildString,
        player: UserId,
    ) -> anyhow::Result<TicTacToeScore> {
        self.access_db(move |db| {
            let txn = db.transaction()?;
            create_user_score_data(&txn, guild_id, player)?;
            let ret = txn.prepare_cached(GET_TIC_TAC_TOE_SCORE_SQL)?.query_row(
                params![guild_id, i64::from(player)],
                |row| {
                    Ok(TicTacToeScore {
                        wins: row.get(0)?,
                        losses: row.get(1)?,
                        ties: row.get(2)?,
                        concedes: row.get(3)?,
                    })
                },
            )?;
            txn.commit().context("failed to commit").map(|_| ret)
        })
        .await?
    }

    /// Get the top Tic-Tac-Toe scores for the current server
    pub async fn get_top_tic_tac_toe_scores(
        &self,
        guild_id: MaybeGuildString,
    ) -> anyhow::Result<Vec<TicTacToeTopPlayerScore>> {
        self.access_db(move |db| {
            let ret = db
                .prepare_cached(GET_TOP_TIC_TAC_TOE_SCORES_SQL)?
                .query_map([guild_id], TicTacToeTopPlayerScore::from_row)?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ret)
        })
        .await?
    }
}
