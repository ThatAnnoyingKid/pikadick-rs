use anyhow::Context;
use fslock::{
    IntoOsString,
    LockFile,
    ToOsStr,
};
use std::sync::Arc;

/// An async `LockFile`.
///
/// Implemented by blocking on the tokio threadpool
#[derive(Debug, Clone)]
pub struct AsyncLockFile {
    file: Arc<tokio::sync::Mutex<LockFile>>,
}

impl AsyncLockFile {
    /// Open a file for locking
    pub async fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: IntoOsString,
    {
        let path = path.into_os_string()?;
        tokio::task::spawn_blocking(move || Self::open_blocking(&path))
            .await
            .context("failed to join task")?
    }

    /// Open a file for locking in a blocking manner
    pub fn open_blocking<P>(path: &P) -> anyhow::Result<Self>
    where
        P: ToOsStr + ?Sized,
    {
        let file = LockFile::open(path)?;
        Ok(Self {
            file: Arc::new(tokio::sync::Mutex::new(file)),
        })
    }

    /// Lock the file
    pub async fn lock(&self) -> anyhow::Result<()> {
        let mut file = self.file.clone().lock_owned().await;
        Ok(tokio::task::spawn_blocking(move || file.lock())
            .await
            .context("failed to join task")??)
    }

    /// Lock the file, writing the PID to it
    pub async fn lock_with_pid(&self) -> anyhow::Result<()> {
        let mut file = self.file.clone().lock_owned().await;
        Ok(tokio::task::spawn_blocking(move || file.lock_with_pid())
            .await
            .context("failed to join task")??)
    }

    /// Try to lock the file, returning `true` if successful.
    pub async fn try_lock(&self) -> anyhow::Result<bool> {
        let mut file = self.file.clone().lock_owned().await;
        Ok(tokio::task::spawn_blocking(move || file.try_lock())
            .await
            .context("failed to join task")??)
    }

    /// Try to lock a file with a pid, returning `true` if successful.
    pub async fn try_lock_with_pid(&self) -> anyhow::Result<bool> {
        let mut file = self.file.clone().lock_owned().await;
        Ok(
            tokio::task::spawn_blocking(move || file.try_lock_with_pid())
                .await
                .context("failed to join task")??,
        )
    }

    /// Try to lock a file with a pid, returning `true` if successful in a blocking manner.
    pub fn try_lock_with_pid_blocking(&self) -> anyhow::Result<bool> {
        Ok(self.file.blocking_lock().try_lock_with_pid()?)
    }

    /// Returns `true` if this owns the lock
    pub async fn owns_lock(&self) -> anyhow::Result<bool> {
        let file = self.file.clone().lock_owned().await;
        tokio::task::spawn_blocking(move || file.owns_lock())
            .await
            .context("failed to join task")
    }

    /// Unlock the file
    pub async fn unlock(&self) -> anyhow::Result<()> {
        let mut file = self.file.clone().lock_owned().await;
        Ok(tokio::task::spawn_blocking(move || file.unlock())
            .await
            .context("failed to join task")??)
    }

    /// Unlock the file in a blocking manner
    pub fn unlock_blocking(&self) -> anyhow::Result<()> {
        Ok(self.file.blocking_lock().unlock()?)
    }
}
