use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use anyhow::Context as _;
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use tracing::error;
use crate::util::AsciiTable;

#[command]
#[description("Get personal stats for Tic-Tac-Toe")]
#[checks(Enabled)]
#[bucket("default")]
pub async fn stats(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let db = client_data.db.clone();
    drop(data_lock);

    let scores = match db
        .get_tic_tac_toe_score(msg.guild_id.into(), msg.author.id)
        .await
        .context("failed to get tic-tac-toe stats")
    {
        Ok(scores) => scores,
        Err(e) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
            return Ok(());
        }
    };

    let mut table = AsciiTable::new(4, 2);

    let mut wins_buffer = itoa::Buffer::new();
    let mut losses_buffer = itoa::Buffer::new();
    let mut ties_buffer = itoa::Buffer::new();
    let mut concedes_buffer = itoa::Buffer::new();

    table.set_cell(0, 0, " Wins ");
    table.set_cell(1, 0, " Losses ");
    table.set_cell(2, 0, " Ties ");
    table.set_cell(3, 0, " Concedes ");

    table.set_cell(0, 1, wins_buffer.format(scores.wins));
    table.set_cell(1, 1, losses_buffer.format(scores.losses));
    table.set_cell(2, 1, ties_buffer.format(scores.ties));
    table.set_cell(3, 1, concedes_buffer.format(scores.concedes));

    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "```\n{}'s Tic-Tac-Toe Stats\n{}\n```",
                msg.author.name, table
            ),
        )
        .await?;
    Ok(())
}
