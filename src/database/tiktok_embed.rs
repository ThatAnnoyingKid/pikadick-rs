use crate::database::{
    model::TikTokEmbedFlags,
    Database,
};
use anyhow::Context;
use rusqlite::{
    named_params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::*;

// Tiktok Embed SQL
const GET_TIKTOK_EMBED_FLAGS_SQL: &str = include_str!("../../sql/get_tiktok_embed_flags.sql");
const SET_TIKTOK_EMBED_FLAGS_SQL: &str = include_str!("../../sql/set_tiktok_embed_flags.sql");

impl Database {
    /// Set the flags for tiktok embeds.
    ///
    /// # Returns
    /// Returns the old flags and new flags in a tuple in that order.
    pub async fn set_tiktok_embed_flags(
        &self,
        guild_id: GuildId,
        set_flags: TikTokEmbedFlags,
        unset_flags: TikTokEmbedFlags,
    ) -> anyhow::Result<(TikTokEmbedFlags, TikTokEmbedFlags)> {
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_flags: TikTokEmbedFlags = txn
                .prepare_cached(GET_TIKTOK_EMBED_FLAGS_SQL)?
                .query_row(
                    named_params! {
                        ":guild_id": i64::from(guild_id),
                    },
                    |row| row.get(0),
                )
                .optional()?
                .unwrap_or_default();

            let mut new_flags = old_flags;
            new_flags.insert(set_flags);
            new_flags.remove(unset_flags);

            txn.prepare_cached(SET_TIKTOK_EMBED_FLAGS_SQL)?
                .execute(named_params! {
                    ":guild_id": i64::from(guild_id),
                    ":flags": new_flags,
                })?;

            txn.commit().context("failed to set tiktok embed")?;

            Ok((old_flags, new_flags))
        })
        .await?
    }

    /// Get the tiktok embed flags.
    pub async fn get_tiktok_embed_flags(
        &self,
        guild_id: GuildId,
    ) -> anyhow::Result<TikTokEmbedFlags> {
        self.access_db(move |db| {
            db.prepare_cached(GET_TIKTOK_EMBED_FLAGS_SQL)?
                .query_row(
                    named_params! {
                        ":guild_id": i64::from(guild_id),
                    },
                    |row| row.get(0),
                )
                .optional()
                .context("failed to read database")
                .map(|v| v.unwrap_or_default())
        })
        .await?
    }
}
