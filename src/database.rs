use serenity::{
    model::prelude::*,
};
use sqlx::{
    SqlitePool,
    Transaction,
};
use std::collections::HashSet;

/// Bincode Error
pub type BincodeError = Box<bincode::ErrorKind>;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// SQLx DB Error
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),

    /// Bincode Ser/De Error
    #[error("{0}")]
    Bincode(#[from] BincodeError),
}

/// Raw key = b"{prefix}0{key}"
fn make_key(prefix: &[u8], key: &[u8]) -> Vec<u8> {
    let mut raw_key = prefix.to_vec();
    raw_key.reserve(1 + key.len());
    raw_key.push(0);
    raw_key.extend(key);

    raw_key
}

#[derive(Clone, Debug)]
pub struct Database {
    db: SqlitePool,
}

impl Database {
    pub async fn new(db: SqlitePool) -> Result<Self, DatabaseError> {
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
        .await?;

        sqlx::query!(
            "
CREATE TABLE IF NOT EXISTS guild_info (
    id INTEGER PRIMARY KEY,
    disabled_commands BLOB
);
            "
        )
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

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
    ) -> Result<(), DatabaseError> {
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
    pub async fn get_disabled_commands(
        &self,
        id: GuildId,
    ) -> Result<HashSet<String>, DatabaseError> {
        let txn = self.db.begin().await?;
        let (data, txn) = get_disabled_commands(txn, id).await?;
        txn.commit().await?;

        Ok(data)
    }

    /// Create the default guild entry for the given GuildId
    async fn create_default_guild_info(&self, id: GuildId) -> Result<(), DatabaseError> {
        let id = id.0 as i64;
        let mut txn = self.db.begin().await?;

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

        txn.commit().await?;

        Ok(())
    }
}

async fn get_disabled_commands(
    mut txn: Transaction<'_, sqlx::Sqlite>,
    id: GuildId,
) -> Result<(HashSet<String>, Transaction<'_, sqlx::Sqlite>), DatabaseError> {
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
        bincode::deserialize(&entry.disabled_commands)?
    };

    Ok((data, txn))
}

#[derive(Debug)]
pub struct Store {
    db: SqlitePool,
    prefix: String,
}

impl Store {
    fn new(db: SqlitePool, prefix: String) -> Self {
        Store { db, prefix }
    }

    pub async fn get<K: AsRef<[u8]>, V: serde::de::DeserializeOwned>(
        &self,
        key: K,
    ) -> Result<Option<V>, DatabaseError> {
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

        let data = bincode::deserialize(&bytes)?;

        Ok(Some(data))
    }

    pub async fn put<K: AsRef<[u8]>, V: serde::Serialize>(
        &self,
        key: K,
        data: V,
    ) -> Result<(), DatabaseError> {
        let key = key.as_ref();

        let key = make_key(self.prefix.as_ref(), key);
        let value = bincode::serialize(&data)?;

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
