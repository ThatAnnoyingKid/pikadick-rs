mod board;
mod concede;
mod move_;
mod play;
mod renderer;
mod scoreboard;
mod stats;

use self::renderer::Renderer;
pub use self::{
    board::board,
    concede::concede,
    move_::move_,
    play::play,
    scoreboard::scoreboard,
    stats::stats,
};
use crate::{
    Database,
    PoiseContext,
    PoiseError,
    database::{
        TicTacToeCreateGameError,
        TicTacToeTryMoveError,
        TicTacToeTryMoveResponse,
        model::{
            TicTacToeGame,
            TicTacToePlayer,
            TicTacToeScore,
            TicTacToeTopPlayerScore,
        },
    },
};
use serenity::{
    model::id::{
        GuildId,
        UserId,
    },
    prelude::Mentionable,
};
use std::sync::Arc;

/// Data pertaining to running tic_tac_toe games
#[derive(Clone)]
pub struct TicTacToeData {
    database: Database,
    renderer: Arc<Renderer>,
}

impl TicTacToeData {
    /// Make a new [`TicTacToeData`].
    pub fn new(database: Database) -> Self {
        let renderer = Renderer::new().expect("Failed to init renderer");

        Self {
            database,
            renderer: Arc::new(renderer),
        }
    }

    /// Render a board.
    async fn render_board(&self, game_board: tic_tac_toe::Board) -> anyhow::Result<Vec<u8>> {
        self.renderer.render_board_async(game_board).await
    }

    /// Create a tic tac toe game.
    async fn create_game(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
        author_team: tic_tac_toe::Team,
        opponent: TicTacToePlayer,
    ) -> Result<TicTacToeGame, TicTacToeCreateGameError> {
        self.database
            .create_tic_tac_toe_game(guild_id.into(), author_id.into(), author_team, opponent)
            .await
    }

    /// Try to make a move.
    async fn try_move(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
        move_index: u8,
    ) -> Result<TicTacToeTryMoveResponse, TicTacToeTryMoveError> {
        self.database
            .try_tic_tac_toe_move(guild_id.into(), author_id.into(), move_index)
            .await
    }

    /// Concede a game.
    async fn concede_game(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
    ) -> anyhow::Result<Option<TicTacToeGame>> {
        self.database
            .concede_tic_tac_toe_game(guild_id.into(), author_id)
            .await
    }

    /// Get a game.
    async fn get_game(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
    ) -> anyhow::Result<Option<TicTacToeGame>> {
        self.database
            .get_tic_tac_toe_game(guild_id.into(), author_id.into())
            .await
    }

    /// Get the score.
    async fn get_score(
        &self,
        guild_id: Option<GuildId>,
        author_id: UserId,
    ) -> anyhow::Result<TicTacToeScore> {
        self.database
            .get_tic_tac_toe_score(guild_id.into(), author_id)
            .await
    }

    /// Get the top scores.
    async fn get_top_scores(
        &self,
        guild_id: GuildId,
    ) -> anyhow::Result<Vec<TicTacToeTopPlayerScore>> {
        self.database.get_top_tic_tac_toe_scores(guild_id).await
    }
}

impl std::fmt::Debug for TicTacToeData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TicTacToeData").finish()
    }
}

impl TicTacToePlayer {
    /// Get the "mention" for a user.
    ///
    /// Computer is "computer" and users are mentioned.
    pub fn mention(self) -> GamePlayerMention {
        GamePlayerMention(self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GamePlayerMention(TicTacToePlayer);

impl std::fmt::Display for GamePlayerMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            TicTacToePlayer::Computer => "computer".fmt(f),
            TicTacToePlayer::User(user_id) => user_id.mention().fmt(f),
        }
    }
}

#[poise::command(
    slash_command,
    rename = "tic-tac-toe",
    description_localized("en-US", "Play a game of Tic-Tac-Toe"),
    subcommands("play", "move_", "concede", "board", "stats", "scoreboard")
)]
pub async fn tic_tac_toe(_ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    Ok(())
}
