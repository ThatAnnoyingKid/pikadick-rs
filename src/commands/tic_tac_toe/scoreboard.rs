use crate::{
    checks::ENABLED_CHECK,
    util::AsciiTable,
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

#[command]
#[description("Get the top stats for Tic-Tac-Toe in this server")]
#[checks(Enabled)]
#[bucket("default")]
pub async fn scoreboard(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock
        .get::<ClientDataKey>()
        .expect("missing client data");
    let db = client_data.db.clone();
    drop(data_lock);

    let scores = match db
        .get_top_tic_tac_toe_scores(msg.guild_id.into())
        .await
        .context("failed to get top tic-tac-toe stats")
    {
        Ok(scores) => scores,
        Err(e) => {
            error!("{:?}", e);
            msg.channel_id.say(&ctx.http, format!("{:?}", e)).await?;
            return Ok(());
        }
    };

    let mut table = AsciiTable::new(7, scores.len() + 1);
    table.set_padding(2);

    table.set_cell(0, 0, "Position");
    table.set_cell(1, 0, "Name");
    table.set_cell(2, 0, "Score");
    table.set_cell(3, 0, "Wins");
    table.set_cell(4, 0, "Losses");
    table.set_cell(5, 0, "Ties");
    table.set_cell(6, 0, "Concedes");

    for (i, score) in scores.iter().enumerate() {
        let username = score
            .player
            .to_user(&ctx)
            .await
            .context("failed to get user name")?
            .name;

        table.set_cell(0, i + 1, format!("{}", i + 1));
        table.set_cell(1, i + 1, username);
        table.set_cell(2, i + 1, score.score.to_string());
        table.set_cell(3, i + 1, score.wins.to_string());
        table.set_cell(4, i + 1, score.losses.to_string());
        table.set_cell(5, i + 1, score.ties.to_string());
        table.set_cell(6, i + 1, score.concedes.to_string());
    }

    msg.channel_id
        .say(
            &ctx.http,
            format!("```\nTop Tic-Tac-Toe Stats\n{}\n```", table),
        )
        .await?;
    Ok(())
}
