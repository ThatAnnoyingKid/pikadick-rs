use anyhow::Context;
use cfg_if::cfg_if;
use std::path::Path;

/// Using the given client, download the file at a url to a given path.
///
/// Note that this function will overwrite the file at the given path.
///
/// # Temporary Files
/// This will create a temp ".part" file in the same directory while downloading.
/// On failure, this file will try to be cleaned up.
/// On success, this temp file will be renamed to the actual file name.
/// As a result, it may be assumed that the file created at the given path is the complete, non-erroneus download.
///
/// # Locking
/// During downloads, the temp file is locked on platforms that support it.
/// If locking is not supported, overwriting a pre-exisiting temp file cause an error.
/// Currently, unix and windows support locking.
pub async fn download_to_path<P>(client: &reqwest::Client, url: &str, path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    // Get the path.
    let path = path.as_ref();

    // Create temp path.
    let temp_path = nd_util::with_push_extension(path, "part");

    // Setup to open the temp file.
    //
    // TODO: On linux, consider probing for O_TMPFILE support somehow and create an unnamed tmp file and use linkat.
    let mut open_options = tokio::fs::OpenOptions::new();
    open_options.write(true);

    // Mandatory, exclusive locking on Windows.
    cfg_if! {
        if #[cfg(windows)] {
            // Ensure that other programs cannot read or write to this one.
            open_options.share_mode(0);
        }
    }

    // If we don't have a mechanism to prevent stomping,
    // at least ensure that we can't stomp.
    cfg_if! {
        if #[cfg(any(windows, unix))] {
            // We prevent stomping by locking somehow.
            // Create and overwrite the temp file.
            open_options.create(true);
        } else {
            // If the temp file exists, return an error.
            open_options.create_new(true);
        }
    }

    // Open the temp file.
    let temp_file = open_options
        .open(&temp_path)
        .await
        .context("failed to create temporary file")?;

    // Create the remove handle for the temp path.
    let mut temp_path = nd_util::DropRemovePath::new(temp_path);

    // Wrap the file in a lock, if the platform supports it.
    cfg_if! {
        if #[cfg(any(unix, windows))] {
            let mut temp_file_lock = fd_lock::RwLock::new(temp_file);
            let mut temp_file = temp_file_lock.try_write().context("failed to lock temp file")?;
        } else {
            let mut temp_file = temp_file;
        }
    }

    // Perform download.
    nd_util::download_to_file(client, url, &mut temp_file)
        .await
        .context("failed to download to file")?;

    // Uwrap the file from the file lock.
    cfg_if! {
        if #[cfg(any(unix, windows))] {
            // Unlock lock.
            drop(temp_file);

            // Get file from lock.
            let temp_file = temp_file_lock.into_inner();
        }
    }

    // Close the file,
    // so that renaming will work on windows as we prevent deleting with a 0 share_mode flag.
    drop(temp_file.into_std());

    // Perform rename from temp file path to actual file path.
    tokio::fs::rename(&temp_path, &path)
        .await
        .context("failed to rename file")?;

    // Persist the file,
    // since it was renamed and we don't want to remove a non-existent file.
    temp_path.persist();

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = reqwest::Client::new();
        download_to_path(&client, "https://google.com", "google.html")
            .await
            .expect("failed to download");
    }
}
