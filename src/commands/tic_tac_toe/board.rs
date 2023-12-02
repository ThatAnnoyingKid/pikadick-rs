use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use serenity::{
    builder::{
        CreateAttachment,
        CreateMessage,
    },
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
#[description("Print the current Tic-Tac-Toe board")]
#[usage("")]
#[example("")]
#[min_args(0)]
#[max_args(0)]
#[bucket("ttt-board")]
#[checks(Enabled)]
pub async fn board(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let tic_tac_toe_data = client_data.tic_tac_toe_data.clone();
    let db = client_data.db.clone();
    drop(data_lock);

    let guild_id = msg.guild_id;
    let author_id = msg.author.id;

    match db
        .get_tic_tac_toe_game(guild_id.into(), author_id.into())
        .await
    {
        Ok(Some(game)) => {
            let file = match tic_tac_toe_data
                .renderer
                .render_board_async(game.board)
                .await
            {
                Ok(file) => {
                    CreateAttachment::bytes(file, format!("ttt-{}.png", game.board.encode_u16()))
                }
                Err(error) => {
                    error!("Failed to render Tic-Tac-Toe board: {error}");
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Failed to render Tic-Tac-Toe board: {error}"),
                        )
                        .await?;
                    return Ok(());
                }
            };
            let message_builder = CreateMessage::new().add_file(file);
            msg.channel_id
                .send_message(&ctx.http, message_builder)
                .await?;
        }
        Ok(None) => {
            let response = "Failed to print board as you have no games in this server".to_string();
            msg.channel_id.say(&ctx.http, response).await?;
            return Ok(());
        }
        Err(error) => {
            error!("{error:?}");
            msg.channel_id.say(&ctx.http, "database error").await?;
        }
    };

    Ok(())
}
