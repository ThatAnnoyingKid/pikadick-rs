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

    /// A db access panicked
    #[error("db access panicked")]
    AccessPanicked(SyncWrapper<Box<dyn std::any::Any + Send>>),
}

/// Copied from tokio
pub struct SyncWrapper<T> {
    value: T,
}

// safety: The SyncWrapper being send allows you to send the inner value across
// thread boundaries.
unsafe impl<T: Send> Send for SyncWrapper<T> {}

// safety: An immutable reference to a SyncWrapper is useless, so moving such an
// immutable reference across threads is safe.
unsafe impl<T> Sync for SyncWrapper<T> {}

impl<T> std::fmt::Debug for SyncWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Add debug bound to impl?
        f.debug_struct("SyncWrapper").finish()
    }
}

impl<T> SyncWrapper<T> {
    /// Make a new [`SyncWrapper`] around a type T
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }

    /// Get the inner value
    pub fn into_inner(self) -> T {
        self.value
    }
}
