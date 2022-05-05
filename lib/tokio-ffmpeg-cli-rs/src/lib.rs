/// Progress event
mod progress_event;

/// The command builder
mod builder;

pub use self::{
    builder::Builder,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

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

    #[tokio::test]
    #[ignore]
    async fn reencode_m3u8() {
        let mut stream = Builder::new()
            .audio_codec("libopus")
            .video_codec("vp9")
            .input(SAMPLE_M3U8)
            .output("reencode_m3u8.webm")
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
