use crate::{
    checks::ENABLED_CHECK,
    database::model::TicTacToePlayer,
    ClientDataKey,
};
use serenity::{
    client::Context,
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
};
use tracing::error;

#[command]
#[description("Concede a game of Tic-Tac-Toe")]
#[usage("")]
#[example("")]
#[min_args(0)]
#[max_args(0)]
#[checks(Enabled)]
#[bucket("default")]
pub async fn concede(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    let game = match db
        .concede_tic_tac_toe_game(guild_id.into(), author_id)
        .await
    {
        Ok(Some(game)) => game,
        Ok(None) => {
            let response = "Failed to concede as you have no games in this server".to_string();
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
        Err(e) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, "database error").await?;
            return Ok(());
        }
    };

    let opponent = game
        .get_opponent(TicTacToePlayer::User(author_id))
        .expect("author is not playing the game");

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
            error!("failed to render Tic-Tac-Toe board: {}", e);
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
        "{} has conceded to {}.",
        author_id.mention(),
        opponent.mention()
    );

    msg.channel_id
        .send_message(&ctx.http, |m| m.content(content).add_file(file))
        .await?;

    Ok(())
}
