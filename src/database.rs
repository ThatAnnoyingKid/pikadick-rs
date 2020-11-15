use serenity::{
    model::prelude::*,
    prelude::Mutex,
};
use sqlx::{
    Connection,
    Executor,
    Transaction,
};
use std::{
    collections::HashSet,
    sync::Arc,
};

pub type BincodeError = Box<bincode::ErrorKind>;

#[derive(Debug)]
pub enum DatabaseError {
    Sqlx(sqlx::Error),
    Bincode(BincodeError),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Sqlx(e) => e.fmt(f),
            DatabaseError::Bincode(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<sqlx::Error> for DatabaseError {
    fn from(e: sqlx::Error) -> Self {
        DatabaseError::Sqlx(e)
    }
}

impl From<BincodeError> for DatabaseError {
    fn from(e: BincodeError) -> Self {
        DatabaseError::Bincode(e)
    }
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
    db: Arc<Mutex<sqlx::sqlite::SqlitePool>>,
}

impl Database {
    pub async fn new(db: sqlx::sqlite::SqlitePool) -> Result<Self, DatabaseError> {
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

        Ok(Database {
            db: Arc::new(Mutex::new(db)),
        })
    }

    // TODO: Consider locking by prefix or exposing some transaction-like interface
    /// Gets the store by name. Locks the entire DB, so use sparingly.
    pub async fn get_store(&self, name: &str) -> Store<'_> {
        Store::new(self.db.lock().await, name.into())
    }

    /// Sets a command as disabled if disabled is true
    pub async fn disable_command(
        &self,
        id: GuildId,
        cmd: &str,
        disable: bool,
    ) -> Result<(), DatabaseError> {
        let db = self.db.lock().await;

        let txn = db.begin().await?;
        let (mut disabled_commands, mut txn) = get_disabled_commands(txn, id).await?;

        if disable {
            disabled_commands.insert(cmd.to_string());
        } else {
            disabled_commands.remove(cmd);
        }

        let data = bincode::serialize(&disabled_commands)?;

        sqlx::query!(
            "
UPDATE guild_info
SET disabled_commands = ?
WHERE id = ?;
                ",
            data,
            id.0 as i64
        )
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

        Ok(())
    }

    pub async fn get_disabled_commands(
        &self,
        id: GuildId,
    ) -> Result<HashSet<String>, DatabaseError> {
        let db = self.db.lock().await;

        let txn = db.begin().await?;
        let (data, txn) = get_disabled_commands(txn, id).await?;
        txn.commit().await?;

        Ok(data)
    }
}

/// Create guild entry if not present
async fn create_default_guild_info<T>(txn: &mut T, id: GuildId) -> Result<(), DatabaseError>
where
    T: Executor<Database = sqlx::Sqlite> + Connection,
{
    // SQLite doesn't support u64, only i64. We cast to i64 to cope,
    // but the id will not match the actual id of the server.
    // This is ok because I'm pretty sure using 'as' is essentially a transmute here.
    // In the future, it might be better to use a byte array or an actual transmute
    sqlx::query!(
        "
INSERT OR IGNORE INTO guild_info (id, disabled_commands) VALUES (?, NULL);
                ",
        id.0 as i64
    )
    .execute(txn)
    .await?;

    Ok(())
}

async fn get_disabled_commands<T>(
    mut txn: Transaction<T>,
    id: GuildId,
) -> Result<(HashSet<String>, Transaction<T>), DatabaseError>
where
    T: Connection<Database = sqlx::Sqlite>,
{
    txn = {
        let mut create_txn = txn.begin().await?;
        create_default_guild_info(&mut create_txn, id).await?;
        create_txn.commit().await?
    };

    let entry = sqlx::query!(
        "
SELECT disabled_commands FROM guild_info WHERE id = ?;
            ",
        id.0 as i64
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

type DbMutexGuard<'a> = tokio::sync::MutexGuard<'a, sqlx::sqlite::SqlitePool>;

#[derive(Debug)]
pub struct Store<'a> {
    db: DbMutexGuard<'a>,
    prefix: String,
}

impl<'a> Store<'a> {
    fn new(db: DbMutexGuard<'a>, prefix: String) -> Self {
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
        .fetch_optional(&*self.db)
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
