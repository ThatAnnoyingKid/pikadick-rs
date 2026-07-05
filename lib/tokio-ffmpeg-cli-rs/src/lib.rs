#![allow(clippy::uninlined_format_args)]

/// The command builder
mod builder;
/// Encoder info
mod encoder;
/// Progress event
mod progress_event;
mod util;

pub use self::{
    builder::Builder,
    encoder::{
        Encoder,
        FromLineError as EncoderFromLineError,
    },
    progress_event::{
        LineBuilderError,
        ProgressEvent,
    },
};
use std::{
    collections::HashMap,
    process::ExitStatus,
};

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

    /// Failed to construct a progress event
    #[error("invalid progress event")]
    InvalidProgressEvent(#[from] crate::progress_event::LineBuilderError),

    /// An exit status was invalid
    #[error("invalid exit status \"{0}\"")]
    InvalidExitStatus(ExitStatus),

    /// Failed to convert bytes to a str
    #[error(transparent)]
    InvalidUtf8Str(std::str::Utf8Error),

    /// Invalid encoder
    #[error("failed to parse encoder line")]
    InvalidEncoderLine(#[from] EncoderFromLineError),

    /// Json error
    #[error("json error")]
    Json(#[from] serde_json::Error),

    /// Utf8 error
    #[error("utf8 error")]
    Utf8(#[from] std::str::Utf8Error),
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

/// Get encoders that this ffmpeg supports
pub async fn get_encoders() -> Result<Vec<Encoder>, Error> {
    let output = tokio::process::Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::InvalidExitStatus(output.status));
    }

    let stdout_str = std::str::from_utf8(&output.stdout).map_err(Error::InvalidUtf8Str)?;
    Ok(stdout_str
        .lines()
        .map(|line| line.trim())
        .skip_while(|line| *line != "------")
        .skip(1)
        .map(Encoder::from_line)
        .collect::<Result<_, _>>()?)
}

/// Result of ffprobe
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProbeResult {
    /// Format info
    pub format: Format,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Format info.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Format {
    /// The # of programs
    pub nb_programs: u32,

    /// The start time, as a string?
    pub start_time: String,

    /// The file name
    pub filename: String,

    /// The bit rate, as a string?
    pub bit_rate: String,

    /// The format name
    pub format_name: String,

    /// The # of streams, as a string
    pub nb_streams: u32,

    /// The long name of the format
    pub format_long_name: String,

    /// The size of the data?
    pub size: String,

    /// The video duration, in seconds.
    #[serde(with = "crate::util::serde::from_str")]
    pub duration: f64,

    /// ?
    pub probe_score: u32,

    /// Extra k/v
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ffprobe a media source.
pub async fn probe(input: &str) -> Result<ProbeResult, Error> {
    let output = tokio::process::Command::new("ffprobe")
        .args(["-v", "error"])
        .arg("-hide_banner")
        .arg(input)
        .args(["-of", "default=noprint_wrappers=0"])
        .args(["-print_format", "json"])
        .arg("-show_format")
        // .args(["-show_entries", "stream"])
        // .arg("-show_programs")
        .output()
        .await
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::InvalidExitStatus(output.status));
    }

    let stdout = std::str::from_utf8(&output.stdout)?;

    Ok(serde_json::from_str(stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use tokio_stream::StreamExt;

    // https://ottverse.com/free-hls-m3u8-test-urls/
    const SAMPLE_M3U8: &str =
        "http://devimages.apple.com.edgekey.net/iphone/samples/bipbop/bipbopall.m3u8";

    // TODO: Unignore
    // This works on my dev machine.
    // This works on my test machine.
    // This works on my deployment machine.
    // However, this fails on CI.
    // The issue is parsing a progress event with out a frame key.
    // I have no idea why this would ever happen.
    #[tokio::test]
    #[ignore]
    async fn transcode_m3u8() -> anyhow::Result<()> {
        let mut stream = Builder::new()
            .audio_codec("copy")
            .video_codec("copy")
            .input(SAMPLE_M3U8)
            .output("transcode_m3u8.mp4")
            .overwrite(true)
            .spawn()
            .context("failed to spawn ffmpeg")?;

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
                Err(error) => {
                    Err(error).context("stream error")?;
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn reencode_m3u8() -> anyhow::Result<()> {
        let mut stream = Builder::new()
            .audio_codec("libopus")
            .video_codec("vp9")
            .input(SAMPLE_M3U8)
            .output("reencode_m3u8.webm")
            .overwrite(true)
            .spawn()
            .context("failed to spawn ffmpeg")?;

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
                Err(error) => {
                    Err(error).context("stream error")?;
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn ffmpeg_get_encoders() -> anyhow::Result<()> {
        let encoders = get_encoders().await.context("failed to get encoders")?;
        dbg!(encoders);

        Ok(())
    }

    #[tokio::test]
    async fn probe_m3u8() -> anyhow::Result<()> {
        let result = probe(SAMPLE_M3U8).await?;
        dbg!(&result);
        Ok(())
    }
}
