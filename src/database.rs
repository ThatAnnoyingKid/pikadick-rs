mod disabled_commands;
mod kv_store;
pub mod model;
mod reddit_embed;
mod tic_tac_toe;
mod tiktok_embed;

pub use self::tic_tac_toe::{
    TicTacToeCreateGameError,
    TicTacToeTryMoveError,
    TicTacToeTryMoveResponse,
};
use anyhow::Context;
use once_cell::sync::Lazy;
use std::{
    os::raw::c_int,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};
use tracing::{
    error,
    warn,
};

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
    db: async_rusqlite::Database,
}

impl Database {
    //// Make a new [`Database`].
    ///
    /// # Safety
    /// This must be called before any other sqlite functions are called.
    pub async unsafe fn new<P>(path: P, create_if_missing: bool) -> anyhow::Result<Self>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        tokio::task::spawn_blocking(move || Self::blocking_new(&path, create_if_missing))
            .await
            .context("failed to join tokio task")?
    }

    /// Make a new [`Database`] in a blocking manner.
    ///
    /// # Safety
    /// This must be called before any other sqlite functions are called.
    pub unsafe fn blocking_new(path: &Path, create_if_missing: bool) -> anyhow::Result<Self> {
        LOGGER_INIT
            .clone()
            .context("failed to init sqlite logger")?;

        let db = async_rusqlite::Database::blocking_open(path, create_if_missing, |db| {
            db.execute_batch(SETUP_TABLES_SQL)
                .context("failed to setup database")?;
            Ok(())
        })
        .context("failed to open database")?;

        Ok(Database { db })
    }

    /// Access the db
    async fn access_db<F, R>(&self, func: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut rusqlite::Connection) -> R + Send + 'static,
        R: Send + 'static,
    {
        Ok(self.db.access_db(move |db| func(db)).await?)
    }

    /// Close the db
    pub async fn close(&self) -> anyhow::Result<()> {
        if let Err(e) = self
            .db
            .access_db(|db| {
                db.execute("PRAGMA OPTIMIZE;", [])?;
                db.execute("VACUUM;", [])
            })
            .await
            .context("failed to access db")
            .and_then(|v| v.context("failed to execute shutdown commands"))
        {
            error!("{}", e);
        }
        self.db
            .close()
            .await
            .context("failed to send close request to db")?;
        self.db.join().await?;

        Ok(())
    }
}
