mod database;

pub use self::database::Database;
pub use rusqlite::{
    self,
    Connection,
};

pub type CloseDbResult = Result<(), (Connection, rusqlite::Error)>;
pub type DbThreadJoinHandle = std::thread::JoinHandle<CloseDbResult>;

pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Rusqlite error
    #[error(transparent)]
    Rusqlite(#[from] rusqlite::Error),

    /// Tokio Join Error
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Failed to send message to db
    #[error("failed to send message to db")]
    SendMessage,

    /// Failed to get result from db
    #[error("failed to get access response from db")]
    MissingResponse(#[source] tokio::sync::oneshot::error::RecvError),

    /// This database was already joined
    #[error("already joined db")]
    AlreadyJoined,

    /// Bad thread join
    #[error("failed to join thread")]
    ThreadJoin,

    /// Setup failed to run
    #[error("init func failed")]
    SetupFunc(#[source] BoxedError),
}
