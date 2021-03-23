use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use log::error;
use minimax::{
    compile_minimax_map,
    tic_tac_toe::TicTacToeIter,
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

type GameStateKey = (Option<GuildId>, UserId);
type ShareGameState = Arc<Mutex<GameState>>;

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    game_states: Arc<Mutex<HashMap<GameStateKey, ShareGameState>>>,
    ai: Arc<MiniMaxAi<TicTacToeRuleSet>>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new() -> Self {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        let ai = Arc::new(MiniMaxAi::new(map));

        Self {
            game_states: Default::default(),
            ai,
        }
    }

    /// Get a game state for a [`GameStateKey`].
    pub fn get_game_state(&self, key: &GameStateKey) -> Option<ShareGameState> {
        self.game_states.lock().get(key).cloned()
    }

    /// Remove a [`GameState`] by key. Returns the [`ShareGameState`] if successful.
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

/// Render a basic Tic-Tac-Toe board.
fn render_board_basic(state: u16) -> String {
    let board_size = 3;
    let reserve_size = 2 * board_size * board_size;
    let start = String::with_capacity(reserve_size);

    (b'0'..b'9')
        .map(char::from)
        .zip(TicTacToeIter::new(state))
        .enumerate()
        .map(|(i, (tile_number, team))| {
            let separator = if (i + 1) % 3 == 0 { '\n' } else { ' ' };

            match team {
                Some(TicTacToeTeam::X) => ['X', separator],
                Some(TicTacToeTeam::O) => ['O', separator],
                None => [tile_number, separator],
            }
        })
        .fold(start, |mut state, el| {
            state.extend(&el);
            state
        })
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

    let move_number = match args.single::<u8>() {
        Ok(num) => num,
        Err(e) => {
            let response = format!("That move is not a number: {}\nUse `tic-tac-toe play <compuer OR @user> <X or O> to start a game.`", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let game_state = match tic_tac_toe_data.get_game_state(&(guild_id, author_id)) {
        Some(game_state) => game_state,
        None => {
            let response = "No games in progress. Make one with `tic-tac-toe play <compuer OR @user> <X or O>`.";
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let (team_turn, player_turn, move_successful, new_state, maybe_winner, opponent, ai_state) = {
        let mut game_state = game_state.lock();
        let team_turn = game_state.get_team_turn();
        let player_turn = game_state.get_player_turn();
        let move_successful = game_state.try_move(team_turn, move_number);

        // Post-move state
        let new_state = game_state.state;
        let maybe_winner = minimax::tic_tac_toe::get_winner(new_state);
        let opponent = game_state.get_player_turn();

        // AI Move
        let ai_state = if move_successful && opponent.is_computer() {
            let ai_state = *tic_tac_toe_data
                .ai
                .get_move(&game_state.state, &team_turn.inverse())
                .expect("invalid game state lookup");
            game_state.state = ai_state;

            Some(ai_state)
        } else {
            None
        };
        let maybe_ai_winner = ai_state.map(minimax::tic_tac_toe::get_winner);

        (
            team_turn,
            player_turn,
            move_successful,
            new_state,
            maybe_winner,
            opponent,
            ai_state,
        )
    };

    if GamePlayer::User(author_id) != player_turn {
        let response = "It is not your turn. Please wait for your opponent to finish.".to_string();
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    if !move_successful {
        let response = format!(
            "Invalid move. Please choose one of the available squares.\n{}",
            render_board_basic(new_state)
        );
        msg.channel_id.say(&ctx.http, response).await?;
        return Ok(());
    }

    if let Some(winner) = maybe_winner {
        let game = tic_tac_toe_data
            .remove_game_state(guild_id, author_id)
            .expect("failed to delete tic-tac-toe game");
        let game = game.lock();

        let winner_player = game.get_player(winner);
        let loser_player = game.get_player(winner.inverse());

        todo!("Announce Winner and loser");
    }

    if minimax::tic_tac_toe::is_tie(new_state) {
        let game = tic_tac_toe_data
            .remove_game_state(guild_id, author_id)
            .expect("failed to delete tic-tac-toe game");

        todo!("Announce both players");
        return Ok(());
    }

    let response = match opponent {
        GamePlayer::User(user_id) => {
            format!(
                "Your turn {}\n{}",
                user_id.mention(),
                render_board_basic(new_state)
            )
        }
        GamePlayer::Computer => {
            if let Some(winner) = ai_state.and_then(minimax::tic_tac_toe::get_winner) {
                todo!("delete game. Announce Winner.");
            }

            todo!("tiecheck. ping author.");
        }
    };

    msg.channel_id.say(&ctx.http, response).await?;

    Ok(())
}

#[command]
#[description("Start a game of Tic-Tac-Toe")]
#[usage("<computer OR @user, X OR O>")]
#[example("computer X")]
#[min_args(2)]
#[max_args(2)]
#[checks(Enabled)]
pub async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    drop(data_lock);

    let opponent: GamePlayer = match args.single() {
        Ok(player) => player,
        Err(e) => {
            let response = format!(
                "Invalid opponent. Choose 'computer' or '@user'. Error: {}",
                e
            );
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let author_team: TicTacToeTeam = match args.single() {
        Ok(team) => team,
        Err(e) => {
            let response = format!("Invalid team. Choose 'X' or 'O'. Error: {}", e);
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let author_id = msg.author.id;
    let guild_id = msg.guild_id;

    let response = {
        let mut game_states = tic_tac_toe_data.game_states.lock();
        let author_in_game = game_states.contains_key(&(msg.guild_id, msg.author.id));
        let opponent_in_game = if let GamePlayer::User(user_id) = opponent {
            game_states.contains_key(&(msg.guild_id, user_id))
        } else {
            false
        };

        if author_in_game {
            "Finish your current game in this server before starting a new one. Use `tic-tac-toe concede` to end your current game.".to_string()
        } else if opponent_in_game {
            "Your opponent is currently in another game in this server. Wait for them to finish."
                .to_string()
        } else {
            let (x_player, o_player) = if author_team == TicTacToeTeam::X {
                (GamePlayer::User(author_id), opponent)
            } else {
                (opponent, GamePlayer::User(author_id))
            };

            let initial_state = 0;
            let mut raw_game = GameState {
                state: initial_state,
                x_player,
                o_player,
            };

            if x_player.is_computer() {
                raw_game.state = *tic_tac_toe_data
                    .ai
                    .get_move(&initial_state, &TicTacToeTeam::X)
                    .expect("failed to calculate first move");
            }

            let game = Arc::new(Mutex::new(raw_game));

            game_states.insert((msg.guild_id, msg.author.id), game.clone());
            if let GamePlayer::User(opponent_id) = opponent {
                game_states.insert((guild_id, author_id), game);

                // Cannot be a computer here as there are at least 2 human players at this point
                let user = if GamePlayer::User(author_id) == x_player {
                    author_id
                } else {
                    opponent_id
                };

                // board state is 0 if both beginning players are users.
                format!(
                    "Game created! Your turn {}\n{}",
                    user.mention(),
                    render_board_basic(initial_state)
                )
            } else {
                // The opponent is not a user, so it is a computer.
                // We already calculated the move and updated if the computer is X.
                // All that's left is to @author and print the board state.

                format!(
                    "Game created! Your turn {}\n{}",
                    author_id.mention(),
                    render_board_basic(initial_state)
                )
            }
        }
    };

    msg.channel_id.say(&ctx.http, response).await?;

    Ok(())
}

#[command]
#[description("Concede a game of Tic-Tac-Toe")]
#[usage("")]
#[example("")]
#[min_args(0)]
#[max_args(0)]
#[checks(Enabled)]
pub async fn concede(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let game_state = match tic_tac_toe_data.remove_game_state(guild_id, author_id) {
        Some(game_state) => game_state,
        None => {
            let response = "Failed to concede as you have no games in this server".to_string();
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
    };

    let opponent = game_state
        .lock()
        .get_opponent(GamePlayer::User(author_id))
        .expect("author is not playing the game");

    let response = match opponent {
        GamePlayer::User(user_id) => {
            format!(
                "{} has conceded to {}.",
                author_id.mention(),
                user_id.mention()
            )
        }
        GamePlayer::Computer => {
            format!("{} has conceded to the computer.", author_id.mention())
        }
    };

    msg.channel_id.say(&ctx.http, response).await?;

    Ok(())
}
