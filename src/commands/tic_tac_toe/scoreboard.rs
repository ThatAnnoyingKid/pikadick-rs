use crate::{
    PoiseContext,
    PoiseError,
    util::AsciiTable,
};
use anyhow::Context as _;
use tracing::error;

#[poise::command(
    slash_command,
    description_localized("en-US", "Get the top stats for Tic-Tac-Toe in this server.")
)]
pub async fn scoreboard(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            ctx.reply("This command can only be used fron a server.")
                .await?;
            return Ok(());
        }
    };

    let scores = match ctx
        .data()
        .tic_tac_toe_data
        .get_top_scores(guild_id)
        .await
        .context("Failed to get top tic-tac-toe stats")
    {
        Ok(scores) => scores,
        Err(error) => {
            error!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;
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

    let response = format!("```\nTop Tic-Tac-Toe Stats\n{table}\n```");

    ctx.reply(response).await?;

    Ok(())
}
