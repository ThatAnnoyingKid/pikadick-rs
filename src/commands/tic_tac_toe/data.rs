use super::{
    GamePlayer,
    Renderer,
    ShareGameState,
    TryMoveError,
    TryMoveResponse,
};
use crate::tic_tac_toe::{
    CreateGameError,
    GameState,
    GameStateKey,
};
use anyhow::Context;
use log::{
    error,
    info,
};
use minimax::{
    compile_minimax_map,
    MiniMaxAi,
    TicTacToeRuleSet,
    TicTacToeTeam,
};
use parking_lot::Mutex;
use serenity::model::prelude::*;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    game_states: Arc<Mutex<HashMap<GameStateKey, ShareGameState>>>,
    ai: Arc<MiniMaxAi<TicTacToeRuleSet>>,
    pub(crate) renderer: Arc<Renderer>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new() -> anyhow::Result<Self> {
        let start = Instant::now();
        info!("Setting up tic-tac-toe AI");

        let map = compile_minimax_map::<TicTacToeRuleSet>();
        let ai = Arc::new(MiniMaxAi::new(map));

        let end = Instant::now();
        info!("Set up tic-tac-toe AI in {:?}", end - start);

        let renderer = Renderer::new().context("failed to initialize tic-tac-toe renderer")?;

        Ok(Self {
            game_states: Default::default(),
            ai,
            renderer: Arc::new(renderer),
        })
    }

    /// Get a game state for a [`GameStateKey`].
    pub fn get_game_state(&self, key: &GameStateKey) -> Option<ShareGameState> {
        self.game_states.lock().get(key).cloned()
    }

    /// Remove a [`GameState`] by key. Returns the [`ShareGameState`] if successful.
    ///
    /// # Deadlocks
    /// This function deadlocks if the game is alreadly locked by the same thread.
    pub fn remove_game_state(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
    ) -> Option<ShareGameState> {
        let mut game_states = self.game_states.lock();

        let shared_game_state = game_states.remove(&(guild_id, author_id))?;

        {
            let game_state = shared_game_state.lock();

            let maybe_opponent = game_state
                .get_opponent(GamePlayer::User(author_id))
                .and_then(GamePlayer::into_user_id);

            if let Some(user_id) = maybe_opponent {
                if game_states.remove(&(guild_id, user_id)).is_none() && user_id != author_id {
                    error!("Tried to delete a non-existent opponent game.");
                }
            }
        }

        Some(shared_game_state)
    }

    /// Create a new [`GameState`].
    pub fn create_game(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
        author_team: TicTacToeTeam,
        opponent: GamePlayer,
    ) -> Result<ShareGameState, CreateGameError> {
        let (x_player, o_player) = if author_team == TicTacToeTeam::X {
            (GamePlayer::User(author_id), opponent)
        } else {
            (opponent, GamePlayer::User(author_id))
        };

        let mut game_states = self.game_states.lock();

        let author_in_game = game_states.contains_key(&(guild_id, author_id));
        let opponent_in_game = opponent.into_user_id().map_or(false, |user_id| {
            game_states.contains_key(&(guild_id, user_id))
        });

        if author_in_game {
            return Err(CreateGameError::AuthorInGame);
        }

        if opponent_in_game {
            return Err(CreateGameError::OpponentInGame);
        }

        let mut raw_game = GameState {
            state: Default::default(),
            x_player,
            o_player,
        };

        if x_player.is_computer() {
            raw_game.state = *self
                .ai
                .get_move(&raw_game.state, &TicTacToeTeam::X)
                .expect("AI failed to calculate the first move");
        }

        let game = Arc::new(Mutex::new(raw_game));
        game_states.insert((guild_id, author_id), game.clone());
        if let GamePlayer::User(opponent_id) = opponent {
            game_states.insert((guild_id, opponent_id), game.clone());
        }

        Ok(game)
    }

    /// Try to make a move.
    pub fn try_move(
        &self,
        game_state: ShareGameState,
        guild_id: Option<GuildId>,
        author_id: UserId,
        move_number: u8,
    ) -> Result<TryMoveResponse, TryMoveError> {
        let mut game_state = game_state.lock();
        let player_turn = game_state.get_player_turn();

        if GamePlayer::User(author_id) != player_turn {
            return Err(TryMoveError::InvalidTurn);
        }

        let team_turn = game_state.get_team_turn();
        let move_successful = game_state.try_move(move_number, team_turn);

        if !move_successful {
            return Err(TryMoveError::InvalidMove);
        }

        if let Some(winner) = game_state.state.get_winning_team() {
            let game = *game_state;
            let winner_player = game.get_player(winner);
            let loser_player = game.get_player(winner.inverse());
            drop(game_state);

            let _game = self
                .remove_game_state(guild_id, author_id)
                .expect("failed to delete tic-tac-toe game");

            return Ok(TryMoveResponse::Winner {
                game,
                winner: winner_player,
                loser: loser_player,
            });
        }

        if game_state.state.is_tie() {
            let game = *game_state;
            drop(game_state);
            let _game = self
                .remove_game_state(guild_id, author_id)
                .expect("failed to delete tic-tac-toe game");

            return Ok(TryMoveResponse::Tie { game });
        }

        let opponent = game_state.get_player_turn();
        if opponent == GamePlayer::Computer {
            let ai_state = *self
                .ai
                .get_move(&game_state.state, &team_turn.inverse())
                .expect("invalid game state lookup");
            game_state.state = ai_state;

            if let Some(winner) = game_state.state.get_winning_team() {
                let game = *game_state;
                let winner_player = game.get_player(winner);
                let loser_player = game.get_player(winner.inverse());
                drop(game_state);

                let _game = self
                    .remove_game_state(guild_id, author_id)
                    .expect("failed to delete tic-tac-toe game");

                return Ok(TryMoveResponse::Winner {
                    game,
                    winner: winner_player,
                    loser: loser_player,
                });
            }

            if game_state.state.is_tie() {
                let game = *game_state;
                drop(game_state);
                let _game = self
                    .remove_game_state(guild_id, author_id)
                    .expect("failed to delete tic-tac-toe game");

                return Ok(TryMoveResponse::Tie { game });
            }
        }

        let game = *game_state;
        Ok(TryMoveResponse::NextTurn { game })
    }
}

impl std::fmt::Debug for TicTacToeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TicTacToeData").finish()
    }
}
