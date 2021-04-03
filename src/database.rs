use anyhow::Context;
use serenity::model::prelude::*;
use sqlx::{
    sqlite::SqliteConnectOptions,
    SqlitePool,
    Transaction,
};
use std::{
    collections::HashSet,
    path::Path,
};

/// Bincode Error type-alias
pub type BincodeError = Box<bincode::ErrorKind>;

/// Turn a prefix + key into  a key for the k/v store.
///
/// # Spec
/// `Raw key = b"{prefix}0{key}"`
fn make_key(prefix: &[u8], key: &[u8]) -> Vec<u8> {
    let mut raw_key = prefix.to_vec();
    raw_key.reserve(1 + key.len());
    raw_key.push(0);
    raw_key.extend(key);

    raw_key
}

/// The database
#[derive(Clone, Debug)]
pub struct Database {
    db: SqlitePool,
}

impl Database {
    //// Make a new [`Database`].
    pub async fn new(db_path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(create_if_missing);
        let db = SqlitePool::connect_with(connect_options)
            .await
            .context("failed to open database")?;

        let mut txn = db.begin().await?;
        sqlx::query!(
            "
CREATE TABLE IF NOT EXISTS kv_store (
    key BLOB PRIMARY KEY,
    value BLOB NOT NULL
);
            "
        )
        .execute(&mut txn)
        .await
        .context("failed to create kv_store table")?;

        sqlx::query!(
            "
CREATE TABLE IF NOT EXISTS guild_info (
    id INTEGER PRIMARY KEY,
    disabled_commands BLOB
);
            "
        )
        .execute(&mut txn)
        .await
        .context("failed to create guild_info table")?;

        txn.commit().await.context("failed to set up database")?;

        Ok(Database { db })
    }

    /// Gets the store by name.
    pub async fn get_store(&self, name: &str) -> Store {
        Store::new(self.db.clone(), name.into())
    }

    /// Sets a command as disabled if disabled is true
    pub async fn disable_command(
        &self,
        id: GuildId,
        cmd: &str,
        disable: bool,
    ) -> anyhow::Result<()> {
        self.create_default_guild_info(id).await?;

        let txn = self.db.begin().await?;
        let (mut disabled_commands, mut txn) = get_disabled_commands(txn, id).await?;

        if disable {
            disabled_commands.insert(cmd.to_string());
        } else {
            disabled_commands.remove(cmd);
        }

        let id = id.0 as i64;
        let data = bincode::serialize(&disabled_commands)?;

        sqlx::query!(
            "
UPDATE guild_info
SET disabled_commands = ?
WHERE id = ?;
            ",
            data,
            id
        )
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

        Ok(())
    }

    /// Get disabled commands as a set
    pub async fn get_disabled_commands(&self, id: GuildId) -> anyhow::Result<HashSet<String>> {
        self.create_default_guild_info(id).await?;
        let txn = self.db.begin().await?;
        let (data, txn) = get_disabled_commands(txn, id)
            .await
            .context("failed to get disabled commands")?;
        txn.commit().await?;

        Ok(data)
    }

    /// Create the default guild entry for the given GuildId
    async fn create_default_guild_info(&self, id: GuildId) -> anyhow::Result<()> {
        let id = id.0 as i64;
        let mut txn = self
            .db
            .begin()
            .await
            .context("failed to start db transaction")?;

        // SQLite doesn't support u64, only i64. We cast to i64 to cope,
        // but the id will not match the actual id of the server.
        // This is ok because I'm pretty sure using 'as' is essentially a transmute here.
        // In the future, it might be better to use a byte array or an actual transmute
        sqlx::query!(
            "
INSERT OR IGNORE INTO guild_info (id, disabled_commands) VALUES (?, NULL);
            ",
            id
        )
        .execute(&mut txn)
        .await?;

        txn.commit()
            .await
            .context("failed to create default guild info")?;

        Ok(())
    }
}

async fn get_disabled_commands(
    mut txn: Transaction<'_, sqlx::Sqlite>,
    id: GuildId,
) -> anyhow::Result<(HashSet<String>, Transaction<'_, sqlx::Sqlite>)> {
    let id = id.0 as i64;
    let entry = sqlx::query!(
        "
SELECT disabled_commands FROM guild_info WHERE id = ?;
            ",
        id
    )
    .fetch_one(&mut txn)
    .await?;

    let data = if entry.disabled_commands.is_empty() {
        HashSet::new()
    } else {
        bincode::deserialize(&entry.disabled_commands)
            .context("failed to decode disabled commands")?
    };

    Ok((data, txn))
}

/// K/V Store
#[derive(Debug)]
pub struct Store {
    db: SqlitePool,
    prefix: String,
}

impl Store {
    fn new(db: SqlitePool, prefix: String) -> Self {
        Store { db, prefix }
    }

    /// Save a key in the store
    pub async fn get<K: AsRef<[u8]>, V: serde::de::DeserializeOwned>(
        &self,
        key: K,
    ) -> anyhow::Result<Option<V>> {
        let key = key.as_ref();

        let key = make_key(self.prefix.as_ref(), key);

        let entry = sqlx::query!(
            "
SELECT * FROM kv_store WHERE key = ?;
            ",
            key
        )
        .fetch_optional(&self.db)
        .await?;

        let bytes = match entry {
            Some(b) => b.value,
            None => {
                return Ok(None);
            }
        };

        let data = bincode::deserialize(&bytes).context("failed to decode value")?;

        Ok(Some(data))
    }

    /// Put a key in the store
    pub async fn put<K: AsRef<[u8]>, V: serde::Serialize>(
        &self,
        key: K,
        data: V,
    ) -> anyhow::Result<()> {
        let key = key.as_ref();

        let key = make_key(self.prefix.as_ref(), key);
        let value = bincode::serialize(&data).context("failed to serialize value")?;

        let mut txn = self.db.begin().await?;
        sqlx::query!(
            "
REPLACE INTO kv_store (key, value)
VALUES(?, ?);
            ",
            key,
            value,
        )
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

        Ok(())
    }
}
