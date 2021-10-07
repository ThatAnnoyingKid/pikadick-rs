use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
    TransactionBehavior,
};
use serenity::model::prelude::*;
use std::{
    path::Path,
    sync::Arc,
};

const SETUP_TABLES_SQL: &str = include_str!("../sql/setup_tables.sql");
const GET_STORE_SQL: &str = "SELECT key_value FROM kv_store WHERE key_prefix = ? AND key_name = ?;";
const PUT_STORE_SQL: &str =
    "INSERT OR REPLACE INTO kv_store (key_prefix, key_name, key_value) VALUES (?, ?, ?);";

/// The database
#[derive(Clone, Debug)]
pub struct Database {
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl Database {
    //// Make a new [`Database`].
    pub async fn new(path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
        let mut flags = rusqlite::OpenFlags::default();
        if !create_if_missing {
            flags.remove(rusqlite::OpenFlags::SQLITE_OPEN_CREATE)
        }
        let path = path.to_owned();
        let db: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let db = rusqlite::Connection::open_with_flags(path, flags)
                .context("failed to open database")?;
            db.execute_batch(SETUP_TABLES_SQL)
                .context("failed to setup database")?;
            Ok(Arc::new(parking_lot::Mutex::new(db)))
        })
        .await?;
        let db = db?;

        Ok(Database { db })
    }

    /// Access the db on the tokio threadpool
    async fn access_db<F, R>(&self, func: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut rusqlite::Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        let db = self.db.clone();
        Ok(tokio::task::spawn_blocking(move || func(&mut db.lock())).await?)
    }

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
        const SELECT_QUERY: &str =
            "SELECT disabled from disabled_commands WHERE guild_id = ? AND name = ?;";
        const UPDATE_QUERY: &str =
            "INSERT OR REPLACE INTO disabled_commands (guild_id, name, disabled) VALUES (?, ?, ?);";

        let cmd = cmd.to_string();
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_value = txn
                .prepare_cached(SELECT_QUERY)?
                .query_row(params![id.0 as i64, cmd], |row| row.get(0))
                .optional()?
                .unwrap_or(false);

            txn.prepare_cached(UPDATE_QUERY)?
                .execute(params![id.0 as i64, cmd, disable])?;
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
            db.prepare_cached(
                "SELECT disabled FROM disabled_commands WHERE guild_id = ? AND name = ?",
            )?
            .query_row(params![id.0 as i64, name], |row| row.get(0))
            .optional()
            .context("failed to access db")
            .map(|row| row.unwrap_or(false))
        })
        .await?
    }

    /// Get a key from the store
    pub async fn store_get<P, K, V>(&self, prefix: P, key: K) -> anyhow::Result<Option<V>>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        let maybe_bytes: Option<Vec<u8>> = self
            .access_db(move |db| {
                db.prepare_cached(GET_STORE_SQL)?
                    .query_row([prefix, key], |row| row.get(0))
                    .optional()
                    .context("failed to get value")
            })
            .await??;

        match maybe_bytes {
            Some(bytes) => Ok(Some(
                bincode::deserialize(&bytes).context("failed to decode value")?,
            )),
            None => Ok(None),
        }
    }

    /// Put a key in the store
    pub async fn store_put<P, K, V>(&self, prefix: P, key: K, value: V) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();
        let value = bincode::serialize(&value).context("failed to serialize value")?;

        self.access_db(move |db| {
            let txn = db.transaction()?;
            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }

    /// Get and Put a key in the store in one action, ensuring the key is not changed between the commands.
    pub async fn store_update<P, K, V, U>(
        &self,
        prefix: P,
        key: K,
        update_func: U,
    ) -> anyhow::Result<()>
    where
        P: AsRef<[u8]>,
        K: AsRef<[u8]>,
        V: serde::Serialize + serde::de::DeserializeOwned,
        U: FnOnce(Option<V>) -> V + Send + 'static,
    {
        let prefix = prefix.as_ref().to_vec();
        let key = key.as_ref().to_vec();

        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;

            let maybe_value = txn
                .prepare_cached(GET_STORE_SQL)?
                .query_row(params![prefix, key], |row| row.get(0))
                .optional()
                .context("failed to get value")?
                .map(|bytes: Vec<u8>| {
                    bincode::deserialize(&bytes).context("failed to decode value")
                })
                .transpose()?;
            let value = update_func(maybe_value);
            let value = bincode::serialize(&value).context("failed to serialize value")?;

            txn.prepare_cached(PUT_STORE_SQL)?
                .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }

    /// Enable or disable reddit embeds.
    ///
    /// # Returns
    /// Returns the old value
    pub async fn set_reddit_embed_enabled(
        &self,
        guild_id: GuildId,
        enabled: bool,
    ) -> anyhow::Result<bool> {
        const SELECT_QUERY: &str =
            "SELECT enabled FROM reddit_embed_guild_settings WHERE guild_id = ?";
        const INSERT_QUERY: &str =
            "INSERT OR REPLACE INTO reddit_embed_guild_settings (guild_id, enabled) VALUES (?, ?);";
        self.access_db(move |db| {
            let txn = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let old_data = txn
                .prepare_cached(SELECT_QUERY)?
                .query_row([guild_id.0 as i64], |row| row.get(0))
                .optional()?
                .unwrap_or(false);
            txn.prepare_cached(INSERT_QUERY)?
                .execute(params![guild_id.0 as i64, enabled])?;

            txn.commit()
                .context("failed to set reddit embed")
                .map(|_| old_data)
        })
        .await?
    }

    /// Get the reddit embed setting.
    pub async fn get_reddit_embed_enabled(&self, guild_id: GuildId) -> anyhow::Result<bool> {
        const SELECT_QUERY: &str =
            "SELECT enabled FROM reddit_embed_guild_settings WHERE guild_id = ?";
        self.access_db(move |db| {
            db.prepare_cached(SELECT_QUERY)?
                .query_row([guild_id.0 as i64], |row| row.get(0))
                .optional()
                .context("failed to read database")
                .map(|v| v.unwrap_or(false))
        })
        .await?
    }
}
