use crate::{
    BoxedError,
    DbThreadJoinHandle,
    Error,
    SyncWrapper,
};
use rusqlite::Connection;
use std::{
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::Arc,
};

const MESSAGE_CHANNEL_SIZE: usize = 128;

enum Message {
    Access {
        func: Box<dyn FnOnce(&mut Connection) + Send + 'static>,
    },
    Close {
        closed: tokio::sync::oneshot::Sender<()>,
    },
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access { .. } => write!(f, "Access"),
            Self::Close { .. } => write!(f, "Close"),
        }
    }
}

/// A database connection
#[derive(Clone)]
pub struct Database {
    sender: tokio::sync::mpsc::Sender<Message>,

    handle: Arc<std::sync::Mutex<Option<DbThreadJoinHandle>>>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Add more data
        f.debug_struct("Database").finish()
    }
}

impl Database {
    /// Open a database at the given path with the setup func.
    pub async fn open<P, S>(path: P, create_if_missing: bool, setup_func: S) -> Result<Self, Error>
    where
        P: Into<PathBuf>,
        S: FnMut(&mut rusqlite::Connection) -> Result<(), BoxedError> + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            Self::open_blocking(path, create_if_missing, setup_func)
        })
        .await?
    }

    /// Open a db in a blocking manner.
    pub fn blocking_open<P, S>(
        path: PathBuf,
        create_if_missing: bool,
        mut setup_func: S,
    ) -> Result<Self, Error>
    where
        P: Into<PathBuf>,
        S: FnMut(&mut rusqlite::Connection) -> Result<(), BoxedError> + Send + 'static,
    {
        // Setup flags
        let mut flags = rusqlite::OpenFlags::default();
        if !create_if_missing {
            flags.remove(rusqlite::OpenFlags::SQLITE_OPEN_CREATE)
        }

        // Open db
        let mut db = Connection::open_with_flags(path, flags)?;

        // Init connection
        setup_func(&mut db).map_err(Error::SetupFunc)?;

        // Setup channel
        let (sender, mut rx) = tokio::sync::mpsc::channel(MESSAGE_CHANNEL_SIZE);

        // Start background handling thread
        let handle = std::thread::spawn(move || {
            while let Some(msg) = rx.blocking_recv() {
                match msg {
                    Message::Access { func } => {
                        func(&mut db);
                    }
                    Message::Close { closed } => {
                        rx.close();

                        // We don't care if a send failed.
                        let _ = closed.send(()).is_ok();
                    }
                }
            }

            // Try close db
            db.close()
        });
        let handle = Arc::new(std::sync::Mutex::new(Some(handle)));

        Ok(Self { sender, handle })
    }

    /// Access the database.
    pub async fn access_db<F, T>(&self, func: F) -> Result<T, Error>
    where
        F: FnOnce(&mut Connection) -> T + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(Message::Access {
                func: Box::new(move |db| {
                    let result = std::panic::catch_unwind(AssertUnwindSafe(|| func(db)));
                    let _ = tx.send(result).is_ok();
                }),
            })
            .await
            .map_err(|_| Error::SendMessage)?;

        rx.await
            .map_err(Error::MissingResponse)?
            .map_err(|e| Error::AccessPanicked(SyncWrapper::new(e)))
    }

    /// Close the db.
    ///
    /// Commands will be able to be queued until this future completes.
    /// Then, all commands that come after will process, though new commands cannot be queued.
    pub async fn close(&self) -> Result<(), Error> {
        let (closed, rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(Message::Close { closed })
            .await
            .map_err(|_| Error::SendMessage)?;
        rx.await.map_err(Error::MissingResponse)
    }

    /// Join background thread.
    ///    
    /// This can only be called once.
    /// Future calls will fail.
    /// You should generally close the db connection before joining.
    pub async fn join(&self) -> Result<(), Error> {
        // Clone to allow user to retry if failed to spawn tokio task.
        let handle = self.handle.clone();
        let result = tokio::task::spawn_blocking(move || {
            handle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
                .ok_or(Error::AlreadyJoined)?
                .join()
                .map_err(|_| Error::ThreadJoin)
        })
        .await??;
        if let Err((_connection, error)) = result {
            return Err(Error::from(error));
        }
        Ok(())
    }
}
