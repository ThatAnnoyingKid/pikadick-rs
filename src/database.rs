mod disabled_commands;
mod kv_store;
pub mod model;
mod reddit_embed;
mod tic_tac_toe;

pub use self::tic_tac_toe::{
    TicTacToeCreateGameError,
    TicTacToeTryMoveError,
    TicTacToeTryMoveResponse,
};
use anyhow::Context;
use once_cell::sync::Lazy;
use std::{
    os::raw::c_int,
    path::Path,
    sync::Arc,
};
use tracing::warn;

// Setup
const SETUP_TABLES_SQL: &str = include_str!("../sql/setup_tables.sql");

static LOGGER_INIT: Lazy<Result<(), Arc<rusqlite::Error>>> = Lazy::new(|| {
    // Safety:
    // 1. `sqlite_logger_func` is threadsafe.
    // 2. This is called only once.
    // 3. This is called before any sqlite functions are used
    // 4. sqlite functions cannot be used until the logger initializes.
    unsafe { rusqlite::trace::config_log(Some(sqlite_logger_func)).map_err(Arc::new) }
});

fn sqlite_logger_func(error_code: c_int, msg: &str) {
    warn!("sqlite error code ({}): {}", error_code, msg);
}

/// The database
#[derive(Clone, Debug)]
pub struct Database {
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
}

impl Database {
    //// Make a new [`Database`].
    pub async fn new(path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
        LOGGER_INIT
            .clone()
            .context("failed to init sqlite logger")?;

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
}
