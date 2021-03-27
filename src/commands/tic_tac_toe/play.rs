use super::{
    GamePlayer,
    GameState,
    TicTacToeData,
};
use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use log::error;
use minimax::TicTacToeTeam;
use parking_lot::Mutex;
use serenity::{
    builder::CreateMessage,
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    http::AttachmentType,
    model::prelude::*,
};
use std::sync::Arc;

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

    let create_message =
        process_tic_tac_toe_play(tic_tac_toe_data, guild_id, author_id, opponent, author_team)
            .await;

    msg.channel_id
        .send_message(&ctx.http, |m| {
            *m = create_message;
            m
        })
        .await?;

    Ok(())
}

async fn process_tic_tac_toe_play(
    tic_tac_toe_data: TicTacToeData,
    guild_id: Option<GuildId>,
    author_id: UserId,
    opponent: GamePlayer,
    author_team: TicTacToeTeam,
) -> CreateMessage<'static> {
    let mut m = CreateMessage::default();
    let game_state;
    let (x_player, o_player) = if author_team == TicTacToeTeam::X {
        (GamePlayer::User(author_id), opponent)
    } else {
        (opponent, GamePlayer::User(author_id))
    };

    {
        let mut game_states = tic_tac_toe_data.game_states.lock();
        let author_in_game = game_states.contains_key(&(guild_id, author_id));
        let opponent_in_game = if let GamePlayer::User(user_id) = opponent {
            game_states.contains_key(&(guild_id, user_id))
        } else {
            false
        };

        if author_in_game {
            let response = "Finish your current game in this server before starting a new one. Use `tic-tac-toe concede` to end your current game.".to_string();
            m.content(response);
            return m;
        }

        if opponent_in_game {
            let response =
            "Your opponent is currently in another game in this server. Wait for them to finish."
                .to_string();
            m.content(response);
            return m;
        }

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
        let game_lock = game.lock();
        game_states.insert((guild_id, author_id), game.clone());

        if let GamePlayer::User(opponent_id) = opponent {
            game_states.insert((guild_id, opponent_id), game.clone());
        }

        game_state = game_lock.state;
    }

    let user = if let GamePlayer::User(opponent_id) = opponent {
        // Cannot be a computer here as there are at least 2 human players at this point
        if GamePlayer::User(author_id) == x_player {
            author_id
        } else {
            opponent_id
        }
    } else {
        // The opponent is not a user, so it is a computer.
        // We already calculated the move and updated if the computer is X.
        // All that's left is to @author and print the board state.
        author_id
    };

    let response = format!("Game created! Your turn {}", user.mention());

    let file = match tic_tac_toe_data
        .renderer
        .render_board_async(game_state)
        .await
    {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to render Tic-Tac-Toe board: {}", e);
            m.content(format!("Failed to render Tic-Tac-Toe board: {}", e));
            return m;
        }
    };

    m.content(response).add_file(AttachmentType::Bytes {
        data: file.into(),
        filename: format!("{}.png", game_state),
    });
    m
}
