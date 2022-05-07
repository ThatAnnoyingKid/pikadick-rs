use std::{
    mem::ManuallyDrop,
    ops::{
        Deref,
        DerefMut,
    },
    path::{
        Path,
        PathBuf,
    },
};
use tokio::fs::File;
use tracing::warn;

/// A [`tokio::fs::File`] wrapper that removes itself on drop
#[derive(Debug)]
pub struct DropRemoveFile {
    /// The file
    file: File,

    /// The path that will remove the file on drop
    path: DropRemovePath,
}

impl DropRemoveFile {
    /// Make a new [`DropRemoveFile`].
    fn new(path: PathBuf, file: File) -> Self {
        Self {
            file,
            path: DropRemovePath::new(path),
        }
    }

    /// Create a file
    pub async fn create<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = File::create(&path).await?;
        Ok(Self::new(path.into(), file))
    }

    /// Persist this file
    pub fn persist(&mut self) {
        self.path.persist();
    }

    /// Close this file, dropping it if needed.
    ///
    /// # Return
    /// Returns an error if the file could not be removed.
    /// Returns Ok(true) if the file was removed
    /// Returns Ok(false) if the file was not removed
    pub async fn close(self) -> Result<bool, (Self, std::io::Error)> {
        self.path.try_drop().await.map_err(|(path, error)| {
            (
                Self {
                    file: self.file,
                    path,
                },
                error,
            )
        })
    }
}

impl Deref for DropRemoveFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for DropRemoveFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

/// Remove a file at a path on drop
#[derive(Debug)]
pub struct DropRemovePath {
    /// The path
    path: PathBuf,

    /// Whether dropping this should remove the file.
    should_remove: bool,
}

impl DropRemovePath {
    /// Make a new [`DropRemovePath`]
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().into(),
            should_remove: true,
        }
    }

    /// Persist this file path
    pub fn persist(&mut self) {
        self.should_remove = false;
    }

    /// Try to drop this file path, removing it if needed.
    ///
    /// # Return
    /// Returns an error if the file could not be removed.
    /// Returns Ok(true) if the file was removed
    /// Returns Ok(false) if the file was not removed
    pub async fn try_drop(self) -> Result<bool, (Self, std::io::Error)> {
        let wrapper = ManuallyDrop::new(self);
        let should_remove = wrapper.should_remove;

        if should_remove {
            tokio::fs::remove_file(&wrapper.path)
                .await
                .map_err(|e| (ManuallyDrop::into_inner(wrapper), e))?;
        }

        Ok(should_remove)
    }
}

impl AsRef<Path> for DropRemovePath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for DropRemovePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for DropRemovePath {
    fn drop(&mut self) {
        let should_remove = self.should_remove;
        let path = std::mem::take(&mut self.path);

        // Try to remove the path.
        tokio::spawn(async move {
            if should_remove {
                if let Err(e) = tokio::fs::remove_file(path).await {
                    warn!("failed to delete file '{}'", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn drop_remove_tokio_file_sanity_check() {
        let file_path: &Path = "./test.txt".as_ref();
        let file_data = b"testing 1 2 3";

        let mut file = DropRemoveFile::create(file_path)
            .await
            .expect("failed to create file");

        file.write_all(file_data)
            .await
            .expect("failed to write data");

        file.close().await.expect("failed to close file");

        let mut file = DropRemoveFile::create(file_path)
            .await
            .expect("failed to create file");

        file.write_all(file_data)
            .await
            .expect("failed to write data");

        file.persist();

        file.close().await.expect("failed to close file");

        let exists = file_path.exists();
        assert!(exists, "persisted file does not exist");

        // Failed cleanup does not matter
        let _ = tokio::fs::remove_file(file_path).await.is_ok();
    }
}
