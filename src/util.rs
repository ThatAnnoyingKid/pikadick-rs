mod arc_anyhow_error;
mod ascii_table;
mod loading_reaction;
mod request_map;
mod timed_cache;

pub use self::{
    arc_anyhow_error::ArcAnyhowError,
    ascii_table::AsciiTable,
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
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio_stream::{
    wrappers::ReceiverStream,
    Stream,
    StreamExt,
};

/// A message for the encoder task
enum EncoderTaskMessage {
    /// Request an encode
    Encode {
        /// The options for the encode
        builder: tokio_ffmpeg_cli::Builder,
        /// The notification for when the task is processed, as well as a handle to the download event stream
        tx: tokio::sync::oneshot::Sender<
            anyhow::Result<
                tokio::sync::mpsc::Receiver<
                    Result<tokio_ffmpeg_cli::Event, tokio_ffmpeg_cli::Error>,
                >,
            >,
        >,
    },

    /// request a shutdown.
    ///
    /// the task will drain the channel until it is empty after recieving this.
    /// the task will still accept new messages until it processes this one.
    Close {
        /// The notification for when the task processes this message
        tx: tokio::sync::oneshot::Sender<()>,
    },
}

/// A task to re-encode things
#[derive(Debug)]
pub struct EncoderTask {
    handle: Arc<parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    tx: tokio::sync::mpsc::Sender<EncoderTaskMessage>,
}

impl EncoderTask {
    /// Make a new encoder task
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(encoder_task_impl(rx));

        Self {
            handle: Arc::new(parking_lot::Mutex::new(Some(handle))),
            tx,
        }
    }

    /// Create a builder for an encode request
    pub fn encode(&self) -> EncoderTaskEncodeBuilder<'_> {
        EncoderTaskEncodeBuilder::new(self)
    }

    /// Request this task to close
    pub async fn close(&self) -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.tx
            .send(EncoderTaskMessage::Close { tx })
            .await
            .ok()
            .context("task is gone")?;

        rx.await.context("task crashed")
    }

    /// Join this task, waiting for it to exit.
    ///
    /// This will NOT send a shutdown request, that must be done beforehand.
    /// Also, this function can only be called once. Future calls will return an error.
    pub async fn join(&self) -> anyhow::Result<()> {
        let handle = self.handle.lock().take().context("missing handle")?;
        handle.await.context("task panicked")
    }

    /// Shutdown the task, sending a close request can joining the task.
    ///
    /// This calls `join` under the hood, so it has the same restrictions as close:
    /// Either shutdown or close can only be called once.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.close().await.context("failed to send close request")?;
        self.join().await.context("failed to join task")?;
        Ok(())
    }
}

/// Impl for the encoder task
async fn encoder_task_impl(mut rx: tokio::sync::mpsc::Receiver<EncoderTaskMessage>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            EncoderTaskMessage::Close { tx } => {
                rx.close();

                // We don't care if the user doesn't care about the result.
                let _ = tx.send(()).is_ok();
            }
            EncoderTaskMessage::Encode { mut builder, tx } => {
                let maybe_stream = builder.spawn().context("failed to spawn FFMpeg");

                match maybe_stream {
                    Ok(mut stream) => {
                        let (event_tx, event_rx) = tokio::sync::mpsc::channel(128);

                        // TODO: Consider stopping download if the user does not care anymore.
                        let _ = tx.send(Ok(event_rx)).is_ok();

                        // We manage the stream here so that downloads are not cancelable.
                        // Also, it gives us a concrete stream type.
                        while let Some(event) = stream.next().await {
                            // TODO: Consider cancelling download if user stopped caring
                            let _ = event_tx.send(event).await.is_ok();
                        }
                    }
                    Err(e) => {
                        // If the stopped caring, we don't care since it was an error anyways
                        let _ = tx.send(Err(e)).is_ok();
                    }
                }
            }
        }
    }
}

impl Default for EncoderTask {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for encoding messages
#[derive(Debug)]
pub struct EncoderTaskEncodeBuilder<'a> {
    builder: tokio_ffmpeg_cli::Builder,

    task: &'a EncoderTask,
}

impl<'a> EncoderTaskEncodeBuilder<'a> {
    /// Make a new [`EncoderTaskEncodeBuilder`]
    pub fn new(task: &'a EncoderTask) -> Self {
        Self {
            builder: tokio_ffmpeg_cli::Builder::new(),
            task,
        }
    }

    /// Try to send the message, exiting it it is at capacity
    pub async fn try_send(
        &self,
    ) -> anyhow::Result<impl Stream<Item = Result<tokio_ffmpeg_cli::Event, tokio_ffmpeg_cli::Error>>>
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.task
            .tx
            .try_send(EncoderTaskMessage::Encode {
                builder: self.builder.clone(),
                tx,
            })
            .ok()
            .context("failed to send message")?;

        rx.await
            .context("encode task crashed")?
            .map(ReceiverStream::new)
    }
}

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

    Ok(())
}
