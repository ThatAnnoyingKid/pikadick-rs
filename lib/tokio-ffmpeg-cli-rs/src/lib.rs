#![allow(clippy::uninlined_format_args)]

/// Progress event
mod progress_event;

/// The command builder
mod builder;

/// Encoder info
mod encoder;

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
use std::process::ExitStatus;

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
    #[error("invalid exit status '{0}'")]
    InvalidExitStatus(ExitStatus),

    /// Failed to convert bytes to a str
    #[error(transparent)]
    InvalidUtf8Str(std::str::Utf8Error),

    /// Invalid encoder
    #[error("failed to parse encoder line")]
    InvalidEncoderLine(#[from] EncoderFromLineError),
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use tokio_stream::StreamExt;

    // https://ottverse.com/free-hls-m3u8-test-urls/
    const SAMPLE_M3U8: &str =
        "http://devimages.apple.com.edgekey.net/iphone/samples/bipbop/bipbopall.m3u8";

    #[tokio::test]
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
}
