use crate::Error;
use std::collections::HashMap;

const FRAME_KEY: &str = "frame";
const FPS_KEY: &str = "fps";
const BITRATE_KEY: &str = "bitrate";
const TOTAL_SIZE_KEY: &str = "total_size";
const OUT_TIME_US_KEY: &str = "out_time_us";
const OUT_TIME_MS_KEY: &str = "out_time_ms";
const OUT_TIME_KEY: &str = "out_time";
const DUP_FRAMES_KEY: &str = "dup_frames";
const DROP_FRAMES_KEY: &str = "drop_frames";
const SPEED_KEY: &str = "speed";
const PROGRESS_KEY: &str = "progress";

/// An event about the encoding progress sent by ffmpeg
#[derive(Debug)]
pub struct ProgressEvent {
    /// The frame number
    pub frame: u64,

    /// The fps
    pub fps: f64,

    /// The bitrate
    pub bitrate: Box<str>,

    /// The progress
    pub progress: Box<str>,

    /// The total size
    pub total_size: u64,

    /// The out time in us
    pub out_time_us: u64,

    /// The out time in ms
    pub out_time_ms: u64,

    /// The out time
    pub out_time: Box<str>,

    /// The # of dup frames
    pub dup_frames: u64,

    /// The # of dropped frames
    pub drop_frames: u64,

    /// The speed.
    /// None means N/A.
    pub speed: Option<f64>,

    /// Extra K/Vs
    pub extra: HashMap<Box<str>, Box<str>>,
}

impl ProgressEvent {
    /// Try to make a [`ProgressEvent`] from optional parts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_from_optional_parts(
        frame: Option<u64>,
        fps: Option<f64>,
        bitrate: Option<Box<str>>,
        total_size: Option<u64>,
        out_time_us: Option<u64>,
        out_time_ms: Option<u64>,
        out_time: Option<Box<str>>,
        dup_frames: Option<u64>,
        drop_frames: Option<u64>,
        speed: Option<Option<f64>>,
        progress: Option<Box<str>>,
        extra: HashMap<Box<str>, Box<str>>,
    ) -> Result<Self, Error> {
        Ok(ProgressEvent {
            frame: frame.ok_or(Error::MissingKeyValuePair(FRAME_KEY))?,
            fps: fps.ok_or(Error::MissingKeyValuePair(FPS_KEY))?,
            bitrate: bitrate.ok_or(Error::MissingKeyValuePair(BITRATE_KEY))?,
            total_size: total_size.ok_or(Error::MissingKeyValuePair(TOTAL_SIZE_KEY))?,
            out_time_us: out_time_us.ok_or(Error::MissingKeyValuePair(OUT_TIME_US_KEY))?,
            out_time_ms: out_time_ms.ok_or(Error::MissingKeyValuePair(OUT_TIME_MS_KEY))?,
            out_time: out_time.ok_or(Error::MissingKeyValuePair(OUT_TIME_KEY))?,
            dup_frames: dup_frames.ok_or(Error::MissingKeyValuePair(DUP_FRAMES_KEY))?,
            drop_frames: drop_frames.ok_or(Error::MissingKeyValuePair(DROP_FRAMES_KEY))?,
            speed: speed.ok_or(Error::MissingKeyValuePair(SPEED_KEY))?,
            progress: progress.ok_or(Error::MissingKeyValuePair(PROGRESS_KEY))?,
            extra,
        })
    }
}

/// A builder to assemble lines into a [`ProgressEvent`].
#[derive(Debug)]
pub(crate) struct ProgressEventLineBuilder {
    maybe_frame: Option<u64>,
    maybe_fps: Option<f64>,
    maybe_bitrate: Option<Box<str>>,
    maybe_total_size: Option<u64>,
    maybe_out_time_us: Option<u64>,
    maybe_out_time_ms: Option<u64>,
    maybe_out_time: Option<Box<str>>,
    maybe_dup_frames: Option<u64>,
    maybe_drop_frames: Option<u64>,
    maybe_speed: Option<Option<f64>>,

    extra: HashMap<Box<str>, Box<str>>,
}

impl ProgressEventLineBuilder {
    /// Make a new [`ProgressEventLineBuilder`].
    pub(crate) fn new() -> Self {
        Self {
            maybe_frame: None,
            maybe_fps: None,
            maybe_bitrate: None,
            maybe_total_size: None,
            maybe_out_time_us: None,
            maybe_out_time_ms: None,
            maybe_out_time: None,
            maybe_dup_frames: None,
            maybe_drop_frames: None,
            maybe_speed: None,

            extra: HashMap::new(),
        }
    }

    /// Push a line to this builder.
    ///
    /// Returns `Some` if the new line would cause the builder to complete a [`ProgressEvent`].
    pub(crate) fn push(&mut self, line: &str) -> Result<Option<ProgressEvent>, Error> {
        let line = line.trim();

        let (key, value) = line.split_once('=').ok_or(Error::InvalidKeyValuePair)?;

        fn parse_kv_u64(
            key: &'static str,
            value: &str,
            store: &mut Option<u64>,
        ) -> Result<(), Error> {
            if store.is_some() {
                return Err(Error::DuplicateKey(key.into()));
            }
            *store = Some(
                value
                    .parse()
                    .map_err(|e| Error::InvalidIntegerValue(key, e))?,
            );
            Ok(())
        }

        fn parse_kv_f64(
            key: &'static str,
            value: &str,
            store: &mut Option<f64>,
        ) -> Result<(), Error> {
            if store.is_some() {
                return Err(Error::DuplicateKey(key.into()));
            }
            *store = Some(
                value
                    .parse()
                    .map_err(|e| Error::InvalidFloatValue(key, e))?,
            );
            Ok(())
        }

        match (key, value) {
            (FRAME_KEY, value) => {
                parse_kv_u64(FRAME_KEY, value, &mut self.maybe_frame)?;
                Ok(None)
            }
            (FPS_KEY, value) => {
                parse_kv_f64(FPS_KEY, value, &mut self.maybe_fps)?;
                Ok(None)
            }
            (BITRATE_KEY, value) => {
                let value = value.trim();

                if self.maybe_bitrate.is_some() {
                    Err(Error::DuplicateKey(BITRATE_KEY.into()))
                } else {
                    self.maybe_bitrate = Some(value.into());
                    Ok(None)
                }
            }
            (TOTAL_SIZE_KEY, value) => {
                parse_kv_u64(TOTAL_SIZE_KEY, value, &mut self.maybe_total_size)?;
                Ok(None)
            }
            (OUT_TIME_US_KEY, value) => {
                parse_kv_u64(OUT_TIME_US_KEY, value, &mut self.maybe_out_time_us)?;
                Ok(None)
            }
            (OUT_TIME_MS_KEY, value) => {
                parse_kv_u64(OUT_TIME_MS_KEY, value, &mut self.maybe_out_time_ms)?;
                Ok(None)
            }
            (OUT_TIME_KEY, out_time) => {
                if self.maybe_out_time.is_some() {
                    Err(Error::DuplicateKey(OUT_TIME_KEY.into()))
                } else {
                    self.maybe_out_time = Some(out_time.into());
                    Ok(None)
                }
            }
            (DUP_FRAMES_KEY, value) => {
                parse_kv_u64(DUP_FRAMES_KEY, value, &mut self.maybe_dup_frames)?;
                Ok(None)
            }
            (DROP_FRAMES_KEY, value) => {
                parse_kv_u64(DROP_FRAMES_KEY, value, &mut self.maybe_drop_frames)?;
                Ok(None)
            }
            (SPEED_KEY, value) => {
                let value = value.trim_end_matches('x').trim_end_matches('X').trim();

                if value == "N/A" {
                    self.maybe_speed = Some(None);
                } else {
                    if self.maybe_speed.is_some() {
                        return Err(Error::DuplicateKey(SPEED_KEY.into()));
                    }
                    self.maybe_speed = Some(Some(
                        value
                            .parse()
                            .map_err(|e| Error::InvalidFloatValue(SPEED_KEY, e))?,
                    ));
                }

                Ok(None)
            }
            (PROGRESS_KEY, progress) => {
                let event = ProgressEvent::try_from_optional_parts(
                    self.maybe_frame.take(),
                    self.maybe_fps.take(),
                    self.maybe_bitrate.take(),
                    self.maybe_total_size.take(),
                    self.maybe_out_time_us.take(),
                    self.maybe_out_time_ms.take(),
                    self.maybe_out_time.take(),
                    self.maybe_dup_frames.take(),
                    self.maybe_drop_frames.take(),
                    self.maybe_speed.take(),
                    Some(progress.into()),
                    std::mem::take(&mut self.extra),
                )?;

                Ok(Some(event))
            }
            (key, value) => {
                if self.extra.insert(key.into(), value.into()).is_some() {
                    Err(Error::DuplicateKey(key.to_string()))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
