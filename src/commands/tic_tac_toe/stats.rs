use crate::{
    PoiseContext,
    PoiseError,
    util::AsciiTable,
};
use anyhow::Context as _;
use tracing::error;

#[poise::command(
    slash_command,
    description_localized("en-US", "Get personal stats for Tic-Tac-Toe.")
)]
pub async fn stats(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    let guild_id = ctx.guild_id();
    let author = ctx.author();
    let author_id = author.id;

    let scores = match ctx
        .data()
        .tic_tac_toe_data
        .get_score(guild_id, author_id)
        .await
        .context("Failed to get tic-tac-toe stats")
    {
        Ok(scores) => scores,
        Err(error) => {
            error!("{error:?}");
            ctx.reply(format!("{error:?}")).await?;
            return Ok(());
        }
    };

    let mut table = AsciiTable::new(4, 2);
    table.set_padding(2);

    let mut wins_buffer = itoa::Buffer::new();
    let mut losses_buffer = itoa::Buffer::new();
    let mut ties_buffer = itoa::Buffer::new();
    let mut concedes_buffer = itoa::Buffer::new();

    table.set_cell(0, 0, "Wins");
    table.set_cell(1, 0, "Losses");
    table.set_cell(2, 0, "Ties");
    table.set_cell(3, 0, "Concedes");

    table.set_cell(0, 1, wins_buffer.format(scores.wins));
    table.set_cell(1, 1, losses_buffer.format(scores.losses));
    table.set_cell(2, 1, ties_buffer.format(scores.ties));
    table.set_cell(3, 1, concedes_buffer.format(scores.concedes));

    let response = format!("```\n{}'s Tic-Tac-Toe Stats\n{}\n```", author.name, table);
    ctx.reply(response).await?;

    Ok(())
}
