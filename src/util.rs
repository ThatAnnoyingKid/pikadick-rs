mod arc_anyhow_error;
mod ascii_table;
mod encoder_task;
mod loading_reaction;
mod request_map;
mod timed_cache;

pub use self::{
    arc_anyhow_error::ArcAnyhowError,
    ascii_table::AsciiTable,
    encoder_task::EncoderTask,
    loading_reaction::LoadingReaction,
    request_map::RequestMap,
    timed_cache::{
        TimedCache,
        TimedCacheEntry,
    },
};
use anyhow::{
    ensure,
    Context,
};
use tokio::io::AsyncWriteExt;

/// Download a url using a GET request to a tokio file.
pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    file: &mut tokio::fs::File,
) -> anyhow::Result<()> {
    // Send the request
    let mut response = client
        .get(url)
        .send()
        .await
        .context("failed to send request")?
        .error_for_status()
        .context("invalid http status")?;

    // Pre-allocate file space if possible.
    let content_length = response.content_length();
    if let Some(content_length) = content_length {
        file.set_len(content_length)
            .await
            .context("failed to pre-allocate file")?;
    }

    // Keep track of the file size in case the server lies
    let mut actual_length = 0;

    // Download the file chunk-by-chunk
    while let Some(chunk) = response.chunk().await.context("failed to get next chunk")? {
        file.write_all(&chunk)
            .await
            .context("failed to write to file")?;
        actual_length +=
            u64::try_from(chunk.len()).context("failed to convert chunk size to `u64`")?;
    }

    // Ensure file size matches content_length
    if let Some(content_length) = content_length {
        ensure!(
            content_length == actual_length,
            "content-length mismatch, {} != {}",
            content_length,
            actual_length
        );
    }

    // Sync data
    file.flush().await.context("failed to flush file")?;
    file.sync_all().await.context("failed to sync file data")?;

    Ok(())
}
