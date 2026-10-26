use crate::database::Database;
use anyhow::Context;
use nd_async_rusqlite::rusqlite::{
    OptionalExtension,
    TransactionBehavior,
    named_params,
};
use serenity::model::prelude::GuildId;

const GET_REDDIT_EMBED_ENABLED_SQL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/sql/get_reddit_embed_enabled.sql"
));
const SET_REDDIT_EMBED_ENABLED_SQL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/sql/set_reddit_embed_enabled.sql"
));

impl Database {
    /// Enable or disable reddit embeds.
    ///
    /// # Returns
    /// Returns the old value.
    pub async fn set_reddit_embed_enabled(
        &self,
        guild_id: GuildId,
        enabled: bool,
    ) -> anyhow::Result<bool> {
        self.write(move |db| {
            let transaction = db.transaction_with_behavior(TransactionBehavior::Immediate)?;

            let old_data = transaction
                .prepare_cached(GET_REDDIT_EMBED_ENABLED_SQL)?
                .query_row(
                    named_params! {
                        ":guild_id": i64::from(guild_id),
                    },
                    |row| row.get(0),
                )
                .optional()?
                .unwrap_or(false);
            transaction
                .prepare_cached(SET_REDDIT_EMBED_ENABLED_SQL)?
                .execute(named_params! {
                    ":guild_id": i64::from(guild_id),
                    ":enabled": enabled,
                })?;

            transaction.commit().context("Failed to set reddit embed")?;

            anyhow::Ok(old_data)
        })
        .await?
    }

    /// Get the reddit embed setting.
    ///
    /// # Returns
    /// This returns an option of a boolean representing whether embeds are enabled for the guild.
    /// If this is None, the caller should pretend that embeds are disabled for the guild.
    pub async fn get_reddit_embed_enabled(
        &self,
        guild_id: GuildId,
    ) -> anyhow::Result<Option<bool>> {
        self.read(move |db| {
            let enabled = db
                .prepare_cached(GET_REDDIT_EMBED_ENABLED_SQL)?
                .query_row(
                    named_params! {
                        ":guild_id": i64::from(guild_id),
                    },
                    |row| row.get(0),
                )
                .optional()
                .context("Failed to read database")?;

            anyhow::Ok(enabled)
        })
        .await?
    }
}
