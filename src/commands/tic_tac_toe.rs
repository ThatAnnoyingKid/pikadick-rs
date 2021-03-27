mod concede;
mod play;
mod tic_tac_toe_renderer;

use self::tic_tac_toe_renderer::TicTacToeRenderer;
pub use self::{
    concede::CONCEDE_COMMAND,
    play::PLAY_COMMAND,
};
use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use log::error;
use minimax::{
    compile_minimax_map,
    MiniMaxAi,
    TicTacToeRuleSet,
    TicTacToeTeam,
};
use parking_lot::Mutex;
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    http::AttachmentType,
    model::{
        channel::Message,
        prelude::*,
    },
    utils::parse_username,
};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
};

/// Error that may occur while creating a game.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum CreateGameError {
    /// The author is in a game
    #[error("the author is in a game")]
    AuthorInGame,

    /// The opponent is in a game
    #[error("the opponent is in a game")]
    OpponentInGame,
}

/// Error that may occur while performing a game move.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum TryMoveError {
    /// It is not the user's turn
    #[error("not the user's turn to move")]
    InvalidTurn,

    /// The move is invalid
    #[error("the move is not valid")]
    InvalidMove,
}

/// The response for making a move.
#[derive(Debug, Copy, Clone)]
pub enum TryMoveResponse {
    Winner {
        game: GameState,
        winner: GamePlayer,
        loser: GamePlayer,
    },
    Tie {
        game: GameState,
    },
    NextTurn {
        game: GameState,
    },
}

type GameStateKey = (Option<GuildId>, UserId);
type ShareGameState = Arc<Mutex<GameState>>;

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    game_states: Arc<Mutex<HashMap<GameStateKey, ShareGameState>>>,
    ai: Arc<MiniMaxAi<TicTacToeRuleSet>>,
    renderer: Arc<TicTacToeRenderer>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new() -> Self {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        let ai = Arc::new(MiniMaxAi::new(map));
        let renderer = TicTacToeRenderer::new().expect("failed to init renderer");

        Self {
            game_states: Default::default(),
            ai,
            renderer: Arc::new(renderer),
        }
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
                .map(GamePlayer::into_user_id)
                .expect("author is not a player in this game");

            if let Some(user_id) = maybe_opponent {
                if game_states.remove(&(guild_id, user_id)).is_none() {
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
            state: 0,
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
        let move_successful = game_state.try_move(team_turn, move_number);

        if !move_successful {
            return Err(TryMoveError::InvalidMove);
        }

        if let Some(winner) = minimax::tic_tac_toe::get_winner(game_state.state) {
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

        if minimax::tic_tac_toe::is_tie(game_state.state) {
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

            if let Some(winner) = minimax::tic_tac_toe::get_winner(game_state.state) {
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

            if minimax::tic_tac_toe::is_tie(game_state.state) {
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

impl Default for TicTacToeData {
    fn default() -> Self {
        Self::new()
    }
}

/// A Tic-Tac-Toe game.
#[derive(Debug, Copy, Clone)]
pub struct GameState {
    /// The Game state
    state: u16,

    /// The X player
    x_player: GamePlayer,

    /// The O player
    o_player: GamePlayer,
}

impl GameState {
    /// Iterate over all [`GamePlayers`].
    ///
    /// Order is X player, O player.
    /// This will include computer players.
    /// Convert players into [`UserId`]s and filter if you want human players.
    pub fn iter_players(&self) -> impl Iterator<Item = GamePlayer> + '_ {
        let mut count = 0;
        std::iter::from_fn(move || {
            let ret = match count {
                0 => self.x_player,
                1 => self.o_player,
                _c => return None,
            };
            count += 1;
            Some(ret)
        })
    }

    /// Get whos turn it is
    pub fn get_team_turn(&self) -> TicTacToeTeam {
        minimax::tic_tac_toe::get_team_turn(self.state)
    }

    /// Get the player whos turn it is
    pub fn get_player_turn(&self) -> GamePlayer {
        let turn = self.get_team_turn();
        match turn {
            TicTacToeTeam::X => self.x_player,
            TicTacToeTeam::O => self.o_player,
        }
    }

    /// Try to make a move. Returns true if successful.
    pub fn try_move(&mut self, team: TicTacToeTeam, tile: u8) -> bool {
        let tile = 3u16.pow(tile.into());

        if ((self.state / tile) % 3) != 0 {
            false
        } else {
            self.state += tile
                * match team {
                    TicTacToeTeam::X => 1,
                    TicTacToeTeam::O => 2,
                };
            true
        }
    }

    /// Get the opponent of the given user in this [`GameState`].
    pub fn get_opponent(&self, player: GamePlayer) -> Option<GamePlayer> {
        match (player == self.x_player, player == self.o_player) {
            (false, false) => None,
            (false, true) => Some(self.x_player),
            (true, false) => Some(self.o_player),
            (true, true) => None,
        }
    }

    /// Get the player for the given team.
    pub fn get_player(&self, team: TicTacToeTeam) -> GamePlayer {
        match team {
            TicTacToeTeam::X => self.x_player,
            TicTacToeTeam::O => self.o_player,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InvalidGamePlayer;

impl std::fmt::Display for InvalidGamePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "invalid player".fmt(f)
    }
}

/// A player of Tic-Tac-Toe
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum GamePlayer {
    /// User
    Computer,

    /// A User
    User(UserId),
}

impl GamePlayer {
    /// Try to convert this into a [`UserId`].
    pub fn into_user_id(self) -> Option<UserId> {
        match self {
            Self::User(id) => Some(id),
            _ => None,
        }
    }

    /// Check if this player is a computer
    pub fn is_computer(self) -> bool {
        matches!(self, Self::Computer)
    }

    /// Get the "mention" for a user.
    ///
    /// Computer is "computer" and users are mentioned.
    pub fn mention(self) -> GamePlayerMention {
        GamePlayerMention(self)
    }
}

impl FromStr for GamePlayer {
    type Err = InvalidGamePlayer;

    fn from_str(data: &str) -> Result<Self, Self::Err> {
        if data.eq_ignore_ascii_case("computer") {
            return Ok(Self::Computer);
        }

        parse_username(data)
            .map(|id| Self::User(UserId(id)))
            .ok_or(InvalidGamePlayer)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GamePlayerMention(GamePlayer);

impl std::fmt::Display for GamePlayerMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            GamePlayer::Computer => "computer".fmt(f),
            GamePlayer::User(user_id) => user_id.mention().fmt(f),
        }
    }
}

#[command("tic-tac-toe")]
#[sub_commands("play", "concede")]
#[description("Play a game of Tic-Tac-Toe")]
#[usage("<move #>")]
#[example("0")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
pub async fn tic_tac_toe(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let move_number = match args.single::<u8>() {
        Ok(num) => num,
        Err(e) => {
            let response = format!("That move is not a number: {}\nUse `tic-tac-toe play <computer/@user> <X/O> to start a game.`", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    if move_number > 8 {
        let response = format!(
            "Your move number must be between 0 and 8 {}",
            author_id.mention()
        );
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    let game_state = match tic_tac_toe_data.get_game_state(&(guild_id, author_id)) {
        Some(game_state) => game_state,
        None => {
            let response =
                "No games in progress. Make one with `tic-tac-toe play <computer/@user> <X/O>`.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    match tic_tac_toe_data.try_move(game_state.clone(), guild_id, author_id, move_number) {
        Ok(TryMoveResponse::Winner {
            game,
            winner,
            loser,
        }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state),
                },
                Err(e) => {
                    error!("Failed to render Tic-Tac-Toe board: {}", e);
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Failed to render Tic-Tac-Toe board: {}", e),
                        )
                        .await?;
                    return Ok(());
                }
            };
            let content = format!(
                "{} has triumphed over {} in Tic-Tac-Toe",
                winner.mention(),
                loser.mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TryMoveResponse::Tie { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state),
                },
                Err(e) => {
                    error!("Failed to render Tic-Tac-Toe board: {}", e);
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Failed to render Tic-Tac-Toe board: {}", e),
                        )
                        .await?;
                    return Ok(());
                }
            };
            let content = format!(
                "{} has tied with {} in Tic-Tac-Toe",
                game.get_player(TicTacToeTeam::X).mention(),
                game.get_player(TicTacToeTeam::O).mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TryMoveResponse::NextTurn { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.state)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.state),
                },
                Err(e) => {
                    error!("Failed to render Tic-Tac-Toe board: {}", e);
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Failed to render Tic-Tac-Toe board: {}", e),
                        )
                        .await?;
                    return Ok(());
                }
            };
            let content = format!("Your turn {}", game.get_player_turn().mention());
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Err(TryMoveError::InvalidTurn) => {
            let response = "It is not your turn. Please wait for your opponent to finish.";
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TryMoveError::InvalidMove) => {
            let response = format!(
                "Invalid move {}. Please choose one of the available squares.\n",
                author_id.mention(),
            );
            msg.channel_id.say(&ctx.http, response).await?;
        }
    }

    Ok(())
}
