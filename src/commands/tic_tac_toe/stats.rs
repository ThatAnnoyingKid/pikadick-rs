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

/// An ascii table
struct Table<'a> {
    data: Vec<Vec<&'a str>>,

    max_cell_widths: Vec<usize>,
}

impl<'a> Table<'a> {
    /// Make a new table
    fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![vec![""; width]; height],
            max_cell_widths: vec![0; width],
        }
    }

    /// Set the value of the given cell.
    ///
    /// Indexing starts at 0. It starts at the top left corner and ends at the bottom right.
    fn set_cell(&mut self, x: usize, y: usize, data: &'a str) {
        self.data[y][x] = data;
        self.max_cell_widths[x] = std::cmp::max(self.max_cell_widths[x], data.len());
    }

    fn fmt_row_border(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+")?;
        for max_cell_width in self.max_cell_widths.iter() {
            for _ in 0..*max_cell_width {
                write!(f, "-")?;
            }
            write!(f, "+")?;
        }
        writeln!(f)?;

        Ok(())
    }
}

impl std::fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.data.iter() {
            self.fmt_row_border(f)?;

            for (cell, max_cell_width) in row.iter().zip(self.max_cell_widths.iter()) {
                let mut padding = 0;
                let cell_len = cell.len();
                if cell_len < *max_cell_width {
                    padding = max_cell_width - cell_len;
                }

                write!(f, "|")?;

                for _ in 0..padding / 2 {
                    write!(f, " ")?;
                }

                write!(f, "{}", cell)?;

                for _ in 0..((padding / 2) + padding % 2) {
                    write!(f, " ")?;
                }
            }
            writeln!(f, "|")?;
        }
        self.fmt_row_border(f)?;

        Ok(())
    }
}

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

    let mut table = Table::new(4, 2);

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

    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "```\n{}'s Tic-Tac-Toe Stats\n{}\n```",
                msg.author.name,
                table
            ),
        )
        .await?;
    Ok(())
}
