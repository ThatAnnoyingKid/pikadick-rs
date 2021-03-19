use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
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
    model::{
        channel::Message,
        prelude::*,
    },
};
use std::{
    collections::{
        hash_map::Entry,
        HashMap,
    },
    sync::Arc,
};

/// Data pertaining to running tic_tac_toe games
///
#[derive(Clone)]
pub struct TicTacToeData {
    game_states: Arc<Mutex<HashMap<UserId, Arc<Mutex<GameState>>>>>,
    ai: Arc<MiniMaxAi<TicTacToeRuleSet>>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    ///
    pub fn new() -> Self {
        let map = compile_minimax_map::<TicTacToeRuleSet>();
        let ai = Arc::new(MiniMaxAi::new(map));

        Self {
            game_states: Default::default(),
            ai,
        }
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

/// A tic tac toe game
///
#[derive(Debug, Copy, Clone)]
pub struct GameState {
    state: u16,
    x_player: GamePlayer,
    y_player: GamePlayer,
}

/// A player of tic_tac_toe
///
#[derive(Debug, Copy, Clone)]
pub enum GamePlayer {
    /// User
    Computer,

    /// A User
    User(UserId),
}

#[command("tic-tac-toe")]
#[sub_commands("play")]
#[description("Play a game of Tic-Tac-Toe")]
#[usage("<X, O, move #>")]
#[example("X")]
#[min_args(1)]
#[max_args(1)]
#[checks(Enabled)]
pub async fn tic_tac_toe(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    drop(data_lock);

    let author_team = args.parse::<TicTacToeTeam>();
    let move_number = args.parse::<u8>();
    args.advance();

    let author_id = msg.author.id;

    let mut error_response = None;
    {
        let mut game_states = tic_tac_toe_data.game_states.lock();
        match game_states.entry(author_id) {
            Entry::Occupied(entry) => {
                todo!()
            }
            Entry::Vacant(entry) => match author_team {
                Ok(author_team) => {
                    let x_player = if author_team == TicTacToeTeam::X {
                        GamePlayer::User(author_id)
                    } else {
                        GamePlayer::Computer
                    };

                    let y_player = if author_team == TicTacToeTeam::O {
                        GamePlayer::User(author_id)
                    } else {
                        GamePlayer::Computer
                    };

                    entry.insert(Arc::new(Mutex::new(GameState {
                        state: 0,
                        x_player,
                        y_player,
                    })));
                }
                Err(e) => {
                    if move_number.is_ok() {
                        error_response =
                            Some("You have already have game in progress.".to_string());
                    } else {
                        error_response = Some(format!(
                            "Failed to create new game (failed to parse team): {}",
                            e
                        ));
                    }
                }
            },
        }
    }

    if let Some(e) = error_response {
        msg.channel_id.say(&ctx.http, e).await?;
    }

    /*
    let team: TicTacToeTeam = match args.single() {
        Ok(team) => team,
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Failed to parse team: {}", e))
                .await?;
            return Ok(());
        }
    };

    */

    Ok(())
}

#[command]
#[description("Start a game of Tic-Tac-Toe")]
#[usage("<computer OR @user, X OR O>")]
#[example("computer X")]
#[min_args(1)]
#[max_args(2)]
#[checks(Enabled)]
pub async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    todo!()
}
