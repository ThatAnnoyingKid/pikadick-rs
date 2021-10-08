mod board;
mod concede;
mod play;
mod renderer;

use self::renderer::Renderer;
pub use self::{
    board::BOARD_COMMAND,
    concede::CONCEDE_COMMAND,
    play::PLAY_COMMAND,
};
use crate::{
    checks::ENABLED_CHECK,
    database::{
        model::{
            TicTacToeGame,
            TicTacToePlayer,
        },
        Database,
        TicTacToeCreateGameError,
        TicTacToeTryMoveError,
        TicTacToeTryMoveResponse,
    },
    ClientDataKey,
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
};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tracing::error;

/// This is the prefix
///
/// # Data Format
/// Key: bincodeify(GameStateKey)
/// Value: bincodify(GameState)
const DATA_STORE_NAME: &str = "tic-tac-toe";

/// A [`GuildId`]/[`UserId`] key to a [`GameState`].
pub type GameStateKey = (Option<GuildId>, UserId);

/// A [`GameState`] that is wrapped in a mutex and sharable via a rc'ed ptr.
pub type ShareGameState = Arc<Mutex<GameState>>;

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    game_states: Arc<Mutex<HashMap<GameStateKey, i64>>>,
    renderer: Arc<Renderer>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new() -> Self {
        let renderer = Renderer::new().expect("failed to init renderer");

        Self {
            game_states: Default::default(),
            renderer: Arc::new(renderer),
        }
    }

    /// Get a game state for a [`GameStateKey`].
    pub fn get_game_state(&self, key: &GameStateKey) -> Option<i64> {
        let id = self.game_states.lock().get(key).cloned(); // TicTacToeGame
                                                            // Some(id)
        id
        /*

        self.access_db(move |db| {
            db.prepare_cached(
                "SELECT board, x_player, o_player FROM tic_tac_toe_games WHERE id = ?;",
            )
            .query_row([id], |row| todo!())
            .unwrap()
        })
        .await
        .unwrap()
        */
    }

    /// Get a game state for a [`GameStateKey`].
    pub fn get_game_state_game(&self, key: &GameStateKey) -> Option<TicTacToeGame> {
        todo!()
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
        /*
        let mut game_states = self.game_states.lock();

        let shared_game_state = game_states.remove(&(guild_id, author_id))?;

        {
            let game_state = shared_game_state.lock();

            let maybe_opponent = game_state
                .get_opponent(TicTacToePlayer::User(author_id))
                .and_then(TicTacToePlayer::get_user);

            if let Some(user_id) = maybe_opponent {
                if game_states.remove(&(guild_id, user_id)).is_none() && user_id != author_id {
                    error!("tried to delete a non-existent opponent game.");
                }
            }
        }

        Some(shared_game_state)
        */
        todo!()
    }

    /// Create a new [`GameState`].
    pub async fn create_game(
        &self,
        db: &Database,
        guild_id: Option<GuildId>,
        author_id: UserId,
        author_team: tic_tac_toe::Team,
        opponent: TicTacToePlayer,
    ) -> Result<(i64, TicTacToeGame), TicTacToeCreateGameError> {
        let (x_player, o_player) = if author_team == tic_tac_toe::Team::X {
            (TicTacToePlayer::User(author_id), opponent)
        } else {
            (opponent, TicTacToePlayer::User(author_id))
        };

        {
            let game_states = self.game_states.lock();

            let author_in_game = game_states.contains_key(&(guild_id, author_id));
            let opponent_in_game = opponent.get_user().map_or(false, |user_id| {
                game_states.contains_key(&(guild_id, user_id))
            });

            if author_in_game {
                return Err(TicTacToeCreateGameError::AuthorInGame);
            }

            if opponent_in_game {
                return Err(TicTacToeCreateGameError::OpponentInGame);
            }
        }

        let (game_id, game) = db.create_tic_tac_toe_game(x_player, o_player).await?;

        let mut game_states = self.game_states.lock();
        game_states.insert((guild_id, author_id), game_id);
        if let TicTacToePlayer::User(opponent_id) = opponent {
            game_states.insert((guild_id, opponent_id), game_id);
        }

        Ok((game_id, game))
    }

    /// Try to make a move.
    pub async fn try_move(
        &self,
        db: &Database,
        game_id: i64,
        guild_id: Option<GuildId>,
        author_id: UserId,
        move_number: u8,
    ) -> Result<TicTacToeTryMoveResponse, TicTacToeTryMoveError> {
        let ret = db
            .try_tic_tac_toe_move(game_id, author_id, move_number)
            .await?;

        if matches!(ret, TicTacToeTryMoveResponse::Tie { .. })
            || matches!(ret, TicTacToeTryMoveResponse::Winner { .. })
        {
            let _game = self
                .remove_game_state(guild_id, author_id)
                .expect("failed to delete tic-tac-toe game");
        }

        Ok(ret)
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
    board: tic_tac_toe::Board,

    /// The X player
    x_player: TicTacToePlayer,

    /// The O player
    o_player: TicTacToePlayer,
}

impl GameState {
    /// Iterate over all [`GamePlayer`]s.
    ///
    /// Order is X player, O player.
    /// This will include computer players.
    /// Convert players into [`UserId`]s and filter if you want human players.
    pub fn iter_players(&self) -> impl Iterator<Item = TicTacToePlayer> + '_ {
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

    /// Get the opponent of the given user in this [`GameState`].
    pub fn get_opponent(&self, player: TicTacToePlayer) -> Option<TicTacToePlayer> {
        match (player == self.x_player, player == self.o_player) {
            (false, false) => None,
            (false, true) => Some(self.x_player),
            (true, false) => Some(self.o_player),
            (true, true) => Some(player), // Player is playing themselves
        }
    }
}

impl TicTacToePlayer {
    /// Get the "mention" for a user.
    ///
    /// Computer is "computer" and users are mentioned.
    pub fn mention(self) -> GamePlayerMention {
        GamePlayerMention(self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GamePlayerMention(TicTacToePlayer);

impl std::fmt::Display for GamePlayerMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            TicTacToePlayer::Computer => "computer".fmt(f),
            TicTacToePlayer::User(user_id) => user_id.mention().fmt(f),
        }
    }
}

#[command("tic-tac-toe")]
#[aliases("ttt")]
#[sub_commands("play", "concede", "board")]
#[description("Play a game of Tic-Tac-Toe")]
#[usage("<move #>")]
#[example("0")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn tic_tac_toe(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let mut move_number = match args.trimmed().single::<u8>() {
        Ok(num) => num,
        Err(e) => {
            let response = format!("That move is not a number: {}\nUse `tic-tac-toe play <computer/@user> <X/O> to start a game.`", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    if !(1..=9).contains(&move_number) {
        let response = format!(
            "Your move number must be between 1 and 9 {}",
            author_id.mention()
        );
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    move_number -= 1;

    let game_state = match tic_tac_toe_data.get_game_state(&(guild_id, author_id)) {
        Some(game_state) => game_state,
        None => {
            let response =
                "No games in progress. Make one with `tic-tac-toe play <computer/@user> <X/O>`.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    match tic_tac_toe_data
        .try_move(&db, game_state, guild_id, author_id, move_number)
        .await
    {
        Ok(TicTacToeTryMoveResponse::Winner {
            game,
            winner,
            loser,
        }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
        Ok(TicTacToeTryMoveResponse::Tie { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
                game.get_player(tic_tac_toe::Team::X).mention(),
                game.get_player(tic_tac_toe::Team::O).mention(),
            );
            msg.channel_id
                .send_message(&ctx.http, |m| m.content(content).add_file(file))
                .await?;
        }
        Ok(TicTacToeTryMoveResponse::NextTurn { game }) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => AttachmentType::Bytes {
                    data: file.into(),
                    filename: format!("ttt-{}.png", game.board.encode_u16()),
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
        Err(TicTacToeTryMoveError::InvalidTurn) => {
            let response = "It is not your turn. Please wait for your opponent to finish.";
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TicTacToeTryMoveError::InvalidMove) => {
            let response = format!(
                "Invalid move {}. Please choose one of the available squares.\n",
                author_id.mention(),
            );
            msg.channel_id.say(&ctx.http, response).await?;
        }
        Err(TicTacToeTryMoveError::Database(e)) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, "database error").await?;
        }
    }

    Ok(())
}
