use anyhow::Context;
use std::{
    ffi::OsString,
    sync::Arc,
};
use tokio::sync::oneshot;
use tokio_stream::{
    wrappers::ReceiverStream,
    Stream,
    StreamExt,
};

/// A message for the encoder task
enum Message {
    /// Request an encode
    Encode {
        /// The options for the encode
        builder: tokio_ffmpeg_cli::Builder,
        /// The notification for when the task is processed, as well as a handle to the download event stream
        tx: oneshot::Sender<
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
        tx: oneshot::Sender<()>,
    },
}

/// A task to re-encode things
#[derive(Debug, Clone)]
pub struct EncoderTask {
    handle: Arc<parking_lot::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    tx: tokio::sync::mpsc::Sender<Message>,
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
        let (tx, rx) = oneshot::channel();

        self.tx
            .send(Message::Close { tx })
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
async fn encoder_task_impl(mut rx: tokio::sync::mpsc::Receiver<Message>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Close { tx } => {
                rx.close();

                // We don't care if the user doesn't care about the result.
                let _ = tx.send(()).is_ok();
            }
            Message::Encode { mut builder, tx } => {
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

    /// Set the file input
    pub fn input(&mut self, input: impl Into<OsString>) -> &mut Self {
        self.builder.input(input);
        self
    }

    /// Set the file output
    pub fn output(&mut self, output: impl Into<OsString>) -> &mut Self {
        self.builder.output(output);
        self
    }

    /// Set the audio codec
    pub fn audio_codec(&mut self, audio_codec: impl Into<String>) -> &mut Self {
        self.builder.audio_codec(audio_codec);
        self
    }

    /// Set the video codec
    pub fn video_codec(&mut self, video_codec: impl Into<String>) -> &mut Self {
        self.builder.video_codec(video_codec);
        self
    }

    /// Try to send the message, exiting it it is at capacity
    pub async fn try_send(
        &self,
    ) -> anyhow::Result<impl Stream<Item = Result<tokio_ffmpeg_cli::Event, tokio_ffmpeg_cli::Error>>>
    {
        let (tx, rx) = oneshot::channel();
        self.task
            .tx
            .try_send(Message::Encode {
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
