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

    /// The path
    path: PathBuf,

    /// Whether dropping this should remove the file.
    should_remove: bool,
}

impl DropRemoveFile {
    /// Make a new [`DropRemoveFile`].
    fn new(path: PathBuf, file: File) -> Self {
        Self {
            file,
            path,
            should_remove: true,
        }
    }

    /// Create a file
    pub async fn create<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let file = File::create(&path).await?;
        Ok(Self::new(path.into(), file))
    }

    /// Persist this file
    pub fn persist(&mut self) {
        self.should_remove = false;
    }

    /// Close this file, dropping it if needed.
    pub async fn close(self) -> Result<(), (Self, std::io::Error)> {
        let wrapper = ManuallyDrop::new(self);

        if wrapper.should_remove {
            tokio::fs::remove_file(&wrapper.path)
                .await
                .map_err(|e| (ManuallyDrop::into_inner(wrapper), e))?;
        }

        Ok(())
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

impl Drop for DropRemoveFile {
    fn drop(&mut self) {
        let should_delete = self.should_remove;
        let path = std::mem::take(&mut self.path);

        tokio::spawn(async move {
            if should_delete {
                if let Err(e) = tokio::fs::remove_file(path).await {
                    warn!("failed to delete file: {}", e);
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
