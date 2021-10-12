use crate::database::Database;
use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::GuildId;

// Disabled Commands SQL
const GET_COMMAND_DISABLED_SQL: &str = include_str!("../../sql/get_command_disabled.sql");
const SET_COMMAND_DISABLED_SQL: &str = include_str!("../../sql/set_command_disabled.sql");

impl Database {
    /// Disables or enables a command.
    ///
    /// # Returns
    /// Returns the old setting
    pub async fn set_disabled_command(
        &self,
        id: GuildId,
        cmd: &str,
        disable: bool,
    ) -> anyhow::Result<bool> {
        let cmd = cmd.to_string();
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_value = txn
                .prepare_cached(GET_COMMAND_DISABLED_SQL)?
                .query_row(params![i64::from(id), cmd], |row| row.get(0))
                .optional()?
                .unwrap_or(false);

            txn.prepare_cached(SET_COMMAND_DISABLED_SQL)?
                .execute(params![i64::from(id), cmd, disable])?;
            txn.commit()
                .context("failed to update disabled command")
                .map(|_| old_value)
        })
        .await?
    }

    /// Check if a command is disabled
    pub async fn is_command_disabled(&self, id: GuildId, name: &str) -> anyhow::Result<bool> {
        let name = name.to_string();
        self.access_db(move |db| {
            db.prepare_cached(GET_COMMAND_DISABLED_SQL)?
                .query_row(params![i64::from(id), name], |row| row.get(0))
                .optional()
                .context("failed to access db")
                .map(|row| row.unwrap_or(false))
        })
        .await?
    }
}
