use crate::database::Database;
use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::GuildId;

// Tiktok Embed SQL
const GET_TIKTOK_EMBED_ENABLED_SQL: &str = include_str!("../../sql/get_tiktok_embed_enabled.sql");
const SET_TIKTOK_EMBED_ENABLED_SQL: &str = include_str!("../../sql/set_tiktok_embed_enabled.sql");

impl Database {
    /// Enable or disable tiktok embeds.
    ///
    /// # Returns
    /// Returns the old value
    pub async fn set_tiktok_embed_enabled(
        &self,
        guild_id: GuildId,
        enabled: bool,
    ) -> anyhow::Result<bool> {
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_data = txn
                .prepare_cached(GET_TIKTOK_EMBED_ENABLED_SQL)?
                .query_row([i64::from(guild_id)], |row| row.get(0))
                .optional()?
                .unwrap_or(false);
            txn.prepare_cached(SET_TIKTOK_EMBED_ENABLED_SQL)?
                .execute(params![i64::from(guild_id), enabled])?;

            txn.commit()
                .context("failed to set tiktok embed")
                .map(|_| old_data)
        })
        .await?
    }

    /// Get the tiktok embed setting.
    pub async fn get_tiktok_embed_enabled(&self, guild_id: GuildId) -> anyhow::Result<bool> {
        self.access_db(move |db| {
            db.prepare_cached(GET_TIKTOK_EMBED_ENABLED_SQL)?
                .query_row([i64::from(guild_id)], |row| row.get(0))
                .optional()
                .context("failed to read database")
                .map(|v| v.unwrap_or(false))
        })
        .await?
    }
}
