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

        let self_clone = self.clone();
        let cmd = cmd.to_string();
        let ret: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let mut db = self_clone.db.lock();
            let txn = db.transaction()?;
            txn.prepare_cached(
                "INSERT OR REPLACE INTO disabled_commands (guild_id, name, disabled) VALUES (?, ?, ?);",
            )?
            .execute(params![id.0, cmd, disable])?;

            txn.commit()?;

            Ok(())
        })
        .await?;
        ret?;

        Ok(())
    }

    /// Get disabled commands as a set
    pub async fn get_disabled_commands(&self, id: GuildId) -> anyhow::Result<HashSet<String>> {
        self.create_default_guild_info(id).await?;

        let self_clone = self.clone();
        let data: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let db = self_clone.db.lock();
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
        let self_clone = self.clone();
        let ret: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let mut db = self_clone.db.lock();

            let id = id.0 as i64;
            let txn = db.transaction()?;
            // SQLite doesn't support u64, only i64. We cast to i64 to cope,
            // but the id will not match the actual id of the server.
            // This is ok because I'm pretty sure using 'as' is essentially a transmute here.
            // In the future, it might be better to use a byte array or an actual transmute
            txn.prepare_cached("INSERT OR IGNORE INTO guilds (id) VALUES (?);")?
                .execute([id])?;
            txn.commit()
                .context("failed to create default guild info")?;
            Ok(())
        })
        .await?;
        ret?;

        Ok(())
    }
}

/*
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
*/

/// K/V Store
#[derive(Debug, Clone)]
pub struct Store {
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
    prefix: String,
}

impl Store {
    fn new(db: Arc<parking_lot::Mutex<rusqlite::Connection>>, prefix: String) -> Self {
        Store { db, prefix }
    }

    /// Save a key in the store
    pub async fn get<K, V>(&self, key: K) -> anyhow::Result<Option<V>>
    where
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        let key = key.as_ref();
        let key = make_key(self.prefix.as_ref(), key);

        let self_clone = self.clone();
        let value: anyhow::Result<_> = tokio::task::spawn_blocking(move || {
            let db = self_clone.db.lock();
            let value: Option<Vec<u8>> = db
                .prepare_cached("SELECT value FROM kv_store WHERE key = ?;")?
                .query_row([key], |row| row.get(0))
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
    pub async fn put<K, V>(&self, key: K, data: V) -> anyhow::Result<()>
    where
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let key = key.as_ref();

        let key = make_key(self.prefix.as_ref(), key);
        let value = bincode::serialize(&data).context("failed to serialize value")?;

        let self_clone = self.clone();
        let ret: anyhow::Result<()> = tokio::task::spawn_blocking(move || {
            let mut db = self_clone.db.lock();
            let txn = db.transaction()?;
            txn.prepare_cached("REPLACE INTO kv_store (key, value) VALUES(?, ?);")?
                .execute(params![key, value])?;
            txn.commit()?;
            Ok(())
        })
        .await?;
        ret?;

        Ok(())
    }
}
