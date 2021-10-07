use anyhow::Context;
use rusqlite::{
    params,
    OptionalExtension,
};
use serenity::model::prelude::*;
use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
};

const SETUP_TABLES_SQL: &str = include_str!("../sql/setup_tables.sql");

/// Bincode Error type-alias
pub type BincodeError = Box<bincode::ErrorKind>;

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

    /// Sets a command as disabled if disabled is true
    pub async fn set_disabled_command(
        &self,
        id: GuildId,
        cmd: &str,
        disable: bool,
    ) -> anyhow::Result<()> {
        self.create_default_guild_info(id).await?;
        let cmd = cmd.to_string();
        self.access_db(move |db| {
            let txn = db.transaction()?;
            txn.prepare_cached(
                "INSERT OR REPLACE INTO disabled_commands (guild_id, name, disabled) VALUES (?, ?, ?);",
            )?
            .execute(params![id.0 as i64, cmd, disable])?;
            txn.commit().context("failed to update disabled command")
        }).await?
    }

    /// Get disabled commands as a set
    pub async fn get_disabled_commands(&self, id: GuildId) -> anyhow::Result<HashSet<String>> {
        self.create_default_guild_info(id).await?;

        let data: anyhow::Result<_> = self
            .access_db(move |db| {
                let set: HashSet<String> = db
                    .prepare_cached(
                        "SELECT name FROM disabled_commands WHERE guild_id = ? AND disabled = 1;",
                    )?
                    .query_map([id.0], |row| row.get(0))?
                    .collect::<Result<_, _>>()?;
                Ok(set)
            })
            .await?;
        let data = data?;

        Ok(data)
    }

    /// Create the default guild entry for the given GuildId
    async fn create_default_guild_info(&self, id: GuildId) -> anyhow::Result<()> {
        self.access_db(move |db| {
            let id = id.0 as i64;
            let txn = db.transaction()?;
            // SQLite doesn't support u64, only i64. We cast to i64 to cope,
            // but the id will not match the actual id of the server.
            // This is ok because I'm pretty sure using 'as' is essentially a transmute here.
            // In the future, it might be better to use a byte array or an actual transmute
            txn.prepare_cached("INSERT OR IGNORE INTO guilds (id) VALUES (?);")?
                .execute([id])?;
            txn.commit().context("failed to create default guild info")
        })
        .await??;

        Ok(())
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

        let value: anyhow::Result<_> = self
            .access_db(move |db| {
                let value: Option<Vec<u8>> = db
                    .prepare_cached(
                        "SELECT key_value FROM kv_store WHERE key_prefix = ? AND key_name = ?;",
                    )?
                    .query_row([prefix, key], |row| row.get(0))
                    .optional()?;
                Ok(value)
            })
            .await?;
        let value = value?;
        let bytes = match value {
            Some(value) => value,
            None => {
                return Ok(None);
            }
        };
        let data = bincode::deserialize(&bytes).context("failed to decode value")?;
        Ok(Some(data))
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
            txn.prepare_cached(
                "INSERT OR REPLACE INTO kv_store (key_prefix, key_name, key_value) VALUES (?, ?, ?);",
            )?
            .execute(params![prefix, key, value])?;
            txn.commit().context("failed to insert key into kv_store")
        })
        .await??;

        Ok(())
    }
}
