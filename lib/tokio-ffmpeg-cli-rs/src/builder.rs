use crate::{
    progress_event::ProgressEventLineBuilder,
    Error,
    Event,
};
use futures::future::FutureExt;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    ffi::OsString,
    process::Stdio,
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

/// Example: "File 'test.mp4' already exists. Exiting."
static FILE_EXISTS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new("File '.*' already exists\\. Exiting\\.")
        .expect("failed to compile FILE_EXISTS_REGEX")
});

/// A builder for an ffmpeg command
#[derive(Debug, Clone)]
pub struct Builder {
    /// The audio codec
    pub audio_codec: Option<String>,

    /// The video codec
    pub video_codec: Option<String>,

    /// The video bitrate
    pub video_bitrate: Option<String>,

    /// The input
    pub input: Option<OsString>,

    /// The output
    pub output: Option<OsString>,

    /// The input format
    pub input_format: Option<String>,

    /// The output format
    pub output_format: Option<String>,

    /// The # of video frames to read from the input
    pub video_frames: Option<u64>,

    /// The video profile
    pub video_profile: Option<String>,

    /// Whether to overwrite the destination
    pub overwrite: bool,
}

impl Builder {
    /// Make a new [`Builder`]
    pub fn new() -> Self {
        Self {
            audio_codec: None,
            video_codec: None,

            video_bitrate: None,

            input: None,
            output: None,

            input_format: None,
            output_format: None,

            video_frames: None,

            video_profile: None,

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

    /// Set the video bitrate
    pub fn video_bitrate(&mut self, video_bitrate: impl Into<String>) -> &mut Self {
        self.video_bitrate = Some(video_bitrate.into());
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

    /// Set the input format
    pub fn input_format(&mut self, input_format: impl Into<String>) -> &mut Self {
        self.input_format = Some(input_format.into());
        self
    }

    /// Set the output format
    pub fn output_format(&mut self, output_format: impl Into<String>) -> &mut Self {
        self.output_format = Some(output_format.into());
        self
    }

    /// The # of video frames to accept from the input
    pub fn video_frames(&mut self, video_frames: impl Into<u64>) -> &mut Self {
        self.video_frames = Some(video_frames.into());
        self
    }

    /// The profile of the video
    pub fn video_profile(&mut self, video_profile: impl Into<String>) -> &mut Self {
        self.video_profile = Some(video_profile.into());
        self
    }

    /// Set whether the output should be overwritten
    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    /// Build the command
    fn build_command(&mut self) -> Result<tokio::process::Command, Error> {
        // https://superuser.com/questions/1459810/how-can-i-get-ffmpeg-command-running-status-in-real-time
        // https://stackoverflow.com/questions/43978018/ffmpeg-get-machine-readable-output
        // https://ffmpeg.org/ffmpeg.html

        let audio_codec = self.audio_codec.take();

        let video_codec = self.video_codec.take();
        let video_bitrate = self.video_bitrate.take();

        let input = self.input.take();
        let output = self.output.take();

        let input_format = self.input_format.take();
        let output_format = self.output_format.take();

        let video_frames = self.video_frames.take();

        let video_profile = self.video_profile.take();

        let overwrite = std::mem::take(&mut self.overwrite);

        let mut command = tokio::process::Command::new("ffmpeg");
        command.arg("-hide_banner");
        command.arg("-nostdin");

        if let Some(input_format) = input_format.as_deref() {
            command.args(["-f", input_format]);
        }

        let input = input.ok_or(Error::MissingInput)?;
        command.args(["-i".as_ref(), input.as_os_str()]);

        if let Some(video_frames) = video_frames {
            // TODO: Consider adding itoa
            command.args(["-frames:v", &video_frames.to_string()]);
        }

        if let Some(audio_codec) = audio_codec.as_deref() {
            command.args(["-codec:a", audio_codec]);
        }

        if let Some(video_codec) = video_codec.as_deref() {
            command.args(["-codec:v", video_codec]);
        }

        if let Some(video_bitrate) = video_bitrate.as_deref() {
            command.args(["-b:v", video_bitrate]);
        }

        if let Some(video_profile) = video_profile.as_deref() {
            command.args(["-profile:v", video_profile]);
        }

        command.args(["-progress", "-"]);
        command.arg(if overwrite { "-y" } else { "-n" });

        if let Some(output_format) = output_format.as_deref() {
            command.args(["-f", output_format]);
        }

        let output = output.ok_or(Error::MissingOutput)?;
        command.arg(output.as_os_str());

        command
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .stderr(Stdio::piped());

        Ok(command)
    }

    /// Run the command and wait for it to finish.
    ///
    /// This will not provide progress info or stdout/stdin, but is far simpler to drive.
    pub async fn ffmpeg_status(&mut self) -> Result<std::process::ExitStatus, Error> {
        self.build_command()?.status().await.map_err(Error::Io)
    }

    /// Run the command and wait for it to finish.
    ///
    /// This will not provide progress info, but is far simpler to drive.
    pub async fn ffmpeg_output(&mut self) -> Result<std::process::Output, Error> {
        self.build_command()?.output().await.map_err(Error::Io)
    }

    /// Spawn the stream
    pub fn spawn(&mut self) -> Result<impl Stream<Item = Result<Event, Error>> + Unpin, Error> {
        let mut command = self.build_command()?;

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
                        .map(|e| e.map(Event::Progress).map_err(From::from))
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
