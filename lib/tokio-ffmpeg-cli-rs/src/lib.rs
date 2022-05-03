/// Events
mod events;

pub use self::events::ProgressEvent;
use crate::events::ProgressEventLineBuilder;
use futures::future::FutureExt;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    ffi::OsString,
    process::{
        ExitStatus,
        Stdio,
    },
};
use tokio::io::{
    AsyncBufReadExt,
    BufReader,
};
use tokio_stream::{
    wrappers::LinesStream,
    Stream,
    StreamExt,
};

// Example: "File 'test.mp4' already exists. Exiting."
static FILE_EXISTS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new("File '.*' already exists\\. Exiting\\.")
        .expect("failed to compile FILE_EXISTS_REGEX")
});

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to spawn a process
    #[error("failed to spawn a process")]
    ProcessSpawn(#[source] std::io::Error),

    /// The input file was not specified
    #[error("missing input file")]
    MissingInput,

    /// The output file was not specified
    #[error("missing output file")]
    MissingOutput,

    /// An IO error occured
    #[error("io error")]
    Io(#[source] std::io::Error),

    /// The output file already exists
    #[error("output file already exists")]
    OutputAlreadyExists,

    /// Invalid integer value for a key
    #[error("invalid integer value for key '{0}'")]
    InvalidIntegerValue(&'static str, std::num::ParseIntError),

    /// Invalid float value for a key
    #[error("invalid float value for key '{0}'")]
    InvalidFloatValue(&'static str, std::num::ParseFloatError),

    /// Got a duplicate key
    #[error("duplicate key '{0}'")]
    DuplicateKey(String),

    /// Missing a key=value pair
    #[error("missing key value pair for key '{0}'")]
    MissingKeyValuePair(&'static str),

    /// The key=value pair is invalid
    #[error("invalid key value pair")]
    InvalidKeyValuePair,
}

/// An Event
#[derive(Debug)]
pub enum Event {
    /// A progress event
    Progress(ProgressEvent),

    /// The process exit status
    ExitStatus(ExitStatus),

    /// An unknown line
    Unknown(String),
}

/// A builder for an ffmpeg command
pub struct Builder {
    /// The audio codec
    pub audio_codec: Option<String>,

    /// The video codec
    pub video_codec: Option<String>,

    /// The input
    pub input: Option<OsString>,

    /// The output
    pub output: Option<OsString>,

    /// Whether to overwrite the destination
    pub overwrite: bool,
}

impl Builder {
    /// Make a new [`Builder`]
    pub fn new() -> Self {
        Self {
            audio_codec: None,
            video_codec: None,

            input: None,
            output: None,

            overwrite: false,
        }
    }

    /// Set the audio codec
    pub fn audio_codec(&mut self, audio_codec: impl Into<String>) -> &mut Self {
        self.audio_codec = Some(audio_codec.into());
        self
    }

    /// Set the video codec
    pub fn video_codec(&mut self, video_codec: impl Into<String>) -> &mut Self {
        self.video_codec = Some(video_codec.into());
        self
    }

    /// Set the input
    pub fn input(&mut self, input: impl Into<OsString>) -> &mut Self {
        self.input = Some(input.into());
        self
    }

    /// Set the output
    pub fn output(&mut self, output: impl Into<OsString>) -> &mut Self {
        self.output = Some(output.into());
        self
    }

    /// Set whether the output should be overwritten
    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    /// Build the command
    pub fn spawn(&mut self) -> Result<impl Stream<Item = Result<Event, Error>> + Unpin, Error> {
        // https://superuser.com/questions/1459810/how-can-i-get-ffmpeg-command-running-status-in-real-time
        // https://stackoverflow.com/questions/43978018/ffmpeg-get-machine-readable-output
        // https://ffmpeg.org/ffmpeg.html

        let audio_codec = self.audio_codec.take();
        let video_codec = self.video_codec.take();
        let input = self.input.take();
        let output = self.output.take();

        let mut command = tokio::process::Command::new("ffmpeg");

        let input = input.ok_or(Error::MissingInput)?;
        command.args(["-i".as_ref(), input.as_os_str()]);

        if let Some(audio_codec) = audio_codec.as_deref() {
            command.args(["-codec:a", audio_codec]);
        }

        if let Some(video_codec) = video_codec.as_deref() {
            command.args(["-codec:v", video_codec]);
        }

        command.args(["-progress", "-"]);
        command.arg(if self.overwrite { "-y" } else { "-n" });

        let output = output.ok_or(Error::MissingOutput)?;
        command.arg(output.as_os_str());

        command
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(Error::ProcessSpawn)?;

        // Stdout Setup
        let stdout = child.stdout.take().expect("missing stdout");
        let stdout_buf_reader = BufReader::new(stdout);
        let stdout_stream = LinesStream::new(stdout_buf_reader.lines());

        // Stderr Setup
        let stderr = child.stderr.take().expect("missing stderr");
        let stderr_buf_reader = BufReader::new(stderr);
        let stderr_stream = LinesStream::new(stderr_buf_reader.lines());

        // Make child produce exit event
        let exit_status_stream = Box::pin(async move { child.wait().await })
            .into_stream()
            .map(|maybe_exit_status| maybe_exit_status.map(Event::ExitStatus).map_err(Error::Io));

        // Process Stdout
        let mut builder = ProgressEventLineBuilder::new();
        let stdout_event_stream = stdout_stream.filter_map(move |maybe_line| {
            let maybe_event = maybe_line
                .map(|line| {
                    builder
                        .push(&line)
                        .transpose()
                        .map(|e| e.map(Event::Progress))
                })
                .transpose()?;

            Some(maybe_event.unwrap_or_else(|e| Err(Error::Io(e))))
        });

        // Process Stderr
        let stderr_event_stream = stderr_stream.map(|maybe_line| {
            let line = maybe_line.map_err(Error::Io)?;

            if FILE_EXISTS_REGEX.is_match(&line) {
                Err(Error::OutputAlreadyExists)
            } else {
                Ok(Event::Unknown(line))
            }
        });

        Ok(stdout_event_stream
            .merge(stderr_event_stream)
            .chain(exit_status_stream))
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://ottverse.com/free-hls-m3u8-test-urls/
    const SAMPLE_M3U8: &str =
        "https://devimages.apple.com.edgekey.net/iphone/samples/bipbop/bipbopall.m3u8";

    #[tokio::test]
    async fn transcode_m3u8() {
        let mut stream = Builder::new()
            .audio_codec("copy")
            .video_codec("copy")
            .input(SAMPLE_M3U8)
            .output("transcode_m3u8.mp4")
            .overwrite(true)
            .spawn()
            .expect("failed to spawn ffmpeg");

        while let Some(maybe_event) = stream.next().await {
            match maybe_event {
                Ok(Event::Progress(event)) => {
                    println!("Progress Event: {:#?}", event);
                }
                Ok(Event::ExitStatus(exit_status)) => {
                    println!("FFMpeg exited: {:?}", exit_status);
                }
                Ok(Event::Unknown(line)) => {
                    //  panic!("{:?}", event);
                    dbg!(line);
                }
                Err(e) => {
                    panic!("Error: {}", e);
                }
            }
        }
    }
}
