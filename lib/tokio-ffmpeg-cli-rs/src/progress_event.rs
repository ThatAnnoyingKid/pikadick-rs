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

const N_A: &str = "N/A";

/// An error that occurs while building a [`ProgressEvent`]
#[derive(Debug, thiserror::Error)]
pub enum LineBuilderError {
    /// The `key=value` pair is invalid
    #[error("invalid key value pair")]
    InvalidKeyValuePair,

    /// Invalid integer value for a key
    #[error("invalid integer value for key \"{key}\" with value \"{value}\"")]
    InvalidIntegerValue {
        key: &'static str,
        value: Box<str>,
        #[source]
        error: std::num::ParseIntError,
    },

    /// Invalid float value for a key
    #[error("invalid float value for key \"{0}\"")]
    InvalidFloatValue(&'static str, #[source] std::num::ParseFloatError),

    /// Got a duplicate key
    #[error("duplicate key \"{key}\"")]
    DuplicateKey {
        /// The duplicate key
        key: Box<str>,
        /// The new duplicate value
        new_value: Box<str>,
        /// The old value
        old_value: Box<str>,
    },

    /// Missing a key=value pair
    #[error("missing key value pair for key \"{0}\"")]
    MissingKeyValuePair(&'static str),
}

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

    /// The total size.
    ///
    /// None means either it was not present, or it was N/A
    pub total_size: Option<u64>,

    /// The out time in us
    pub out_time_us: i64,

    /// The out time in ms
    pub out_time_ms: i64,

    /// The out time
    pub out_time: Box<str>,

    /// The # of dup frames
    pub dup_frames: u64,

    /// The # of dropped frames
    pub drop_frames: u64,

    /// The speed.
    ///
    /// None means it was N/A.
    pub speed: Option<f64>,

    /// Extra K/Vs
    pub extra: HashMap<Box<str>, Box<str>>,
}

impl ProgressEvent {
    /// Try to make a [`ProgressEvent`] from optional parts.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_from_optional_parts(
        frame: Option<u64>,
        fps: Option<f64>,
        bitrate: Option<Box<str>>,
        total_size: Option<Option<u64>>,
        out_time_us: Option<i64>,
        out_time_ms: Option<i64>,
        out_time: Option<Box<str>>,
        dup_frames: Option<u64>,
        drop_frames: Option<u64>,
        speed: Option<Option<f64>>,
        progress: Option<Box<str>>,
        extra: HashMap<Box<str>, Box<str>>,
    ) -> Result<Self, LineBuilderError> {
        Ok(ProgressEvent {
            frame: frame.ok_or(LineBuilderError::MissingKeyValuePair(FRAME_KEY))?,
            fps: fps.ok_or(LineBuilderError::MissingKeyValuePair(FPS_KEY))?,
            bitrate: bitrate.ok_or(LineBuilderError::MissingKeyValuePair(BITRATE_KEY))?,
            total_size: total_size.ok_or(LineBuilderError::MissingKeyValuePair(TOTAL_SIZE_KEY))?,
            out_time_us: out_time_us
                .ok_or(LineBuilderError::MissingKeyValuePair(OUT_TIME_US_KEY))?,
            out_time_ms: out_time_ms
                .ok_or(LineBuilderError::MissingKeyValuePair(OUT_TIME_MS_KEY))?,
            out_time: out_time.ok_or(LineBuilderError::MissingKeyValuePair(OUT_TIME_KEY))?,
            dup_frames: dup_frames.ok_or(LineBuilderError::MissingKeyValuePair(DUP_FRAMES_KEY))?,
            drop_frames: drop_frames
                .ok_or(LineBuilderError::MissingKeyValuePair(DROP_FRAMES_KEY))?,
            speed: speed.ok_or(LineBuilderError::MissingKeyValuePair(SPEED_KEY))?,
            progress: progress.ok_or(LineBuilderError::MissingKeyValuePair(PROGRESS_KEY))?,
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
    maybe_total_size: Option<Option<u64>>,
    maybe_out_time_us: Option<i64>,
    maybe_out_time_ms: Option<i64>,
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
    pub(crate) fn push(&mut self, line: &str) -> Result<Option<ProgressEvent>, LineBuilderError> {
        let line = line.trim();

        let (key, value) = line
            .split_once('=')
            .ok_or(LineBuilderError::InvalidKeyValuePair)?;

        fn parse_kv_u64(
            key: &'static str,
            value: &str,
            store: &mut Option<u64>,
        ) -> Result<(), LineBuilderError> {
            if let Some(store) = store {
                return Err(LineBuilderError::DuplicateKey {
                    key: key.into(),
                    new_value: value.into(),
                    old_value: store.to_string().into(),
                });
            }
            *store =
                Some(
                    value
                        .parse()
                        .map_err(|error| LineBuilderError::InvalidIntegerValue {
                            key,
                            value: value.into(),
                            error,
                        })?,
                );
            Ok(())
        }

        fn parse_kv_i64(
            key: &'static str,
            value: &str,
            store: &mut Option<i64>,
        ) -> Result<(), LineBuilderError> {
            if let Some(store) = store {
                return Err(LineBuilderError::DuplicateKey {
                    key: key.into(),
                    new_value: value.into(),
                    old_value: store.to_string().into(),
                });
            }
            *store =
                Some(
                    value
                        .parse()
                        .map_err(|error| LineBuilderError::InvalidIntegerValue {
                            key,
                            value: value.into(),
                            error,
                        })?,
                );
            Ok(())
        }

        fn parse_kv_f64(
            key: &'static str,
            value: &str,
            store: &mut Option<f64>,
        ) -> Result<(), LineBuilderError> {
            if let Some(store) = store {
                return Err(LineBuilderError::DuplicateKey {
                    key: key.into(),
                    old_value: store.to_string().into(),
                    new_value: value.into(),
                });
            }
            *store = Some(
                value
                    .parse()
                    .map_err(|e| LineBuilderError::InvalidFloatValue(key, e))?,
            );
            Ok(())
        }

        match key {
            FRAME_KEY => {
                parse_kv_u64(FRAME_KEY, value, &mut self.maybe_frame)?;
                Ok(None)
            }
            FPS_KEY => {
                parse_kv_f64(FPS_KEY, value, &mut self.maybe_fps)?;
                Ok(None)
            }
            BITRATE_KEY => {
                let value = value.trim();

                if let Some(old_value) = self.maybe_bitrate.take() {
                    Err(LineBuilderError::DuplicateKey {
                        key: BITRATE_KEY.into(),
                        old_value,
                        new_value: value.into(),
                    })
                } else {
                    self.maybe_bitrate = Some(value.into());
                    Ok(None)
                }
            }
            TOTAL_SIZE_KEY => {
                if value == N_A {
                    self.maybe_total_size = Some(None);
                } else {
                    if let Some(maybe_total_size) = self.maybe_total_size {
                        return Err(LineBuilderError::DuplicateKey {
                            key: TOTAL_SIZE_KEY.into(),
                            old_value: maybe_total_size
                                .map(|v| Box::<str>::from(v.to_string()))
                                .unwrap_or_else(|| N_A.into()),
                            new_value: value.into(),
                        });
                    }
                    self.maybe_total_size = Some(Some(value.parse().map_err(|error| {
                        LineBuilderError::InvalidIntegerValue {
                            key: TOTAL_SIZE_KEY,
                            value: value.into(),
                            error,
                        }
                    })?));
                }

                Ok(None)
            }
            OUT_TIME_US_KEY => {
                parse_kv_i64(OUT_TIME_US_KEY, value, &mut self.maybe_out_time_us)?;
                Ok(None)
            }
            OUT_TIME_MS_KEY => {
                parse_kv_i64(OUT_TIME_MS_KEY, value, &mut self.maybe_out_time_ms)?;
                Ok(None)
            }
            OUT_TIME_KEY => {
                if let Some(old_value) = self.maybe_out_time.take() {
                    Err(LineBuilderError::DuplicateKey {
                        key: OUT_TIME_KEY.into(),
                        old_value,
                        new_value: value.into(),
                    })
                } else {
                    self.maybe_out_time = Some(value.into());
                    Ok(None)
                }
            }
            DUP_FRAMES_KEY => {
                parse_kv_u64(DUP_FRAMES_KEY, value, &mut self.maybe_dup_frames)?;
                Ok(None)
            }
            DROP_FRAMES_KEY => {
                parse_kv_u64(DROP_FRAMES_KEY, value, &mut self.maybe_drop_frames)?;
                Ok(None)
            }
            SPEED_KEY => {
                let value = value.trim_end_matches('x').trim_end_matches('X').trim();

                if value == N_A {
                    self.maybe_speed = Some(None);
                } else {
                    if let Some(maybe_speed) = self.maybe_speed {
                        return Err(LineBuilderError::DuplicateKey {
                            key: SPEED_KEY.into(),
                            old_value: maybe_speed
                                .map(|v| Box::<str>::from(v.to_string()))
                                .unwrap_or_else(|| N_A.into()),
                            new_value: value.into(),
                        });
                    }
                    self.maybe_speed =
                        Some(Some(value.parse().map_err(|e| {
                            LineBuilderError::InvalidFloatValue(SPEED_KEY, e)
                        })?));
                }

                Ok(None)
            }
            PROGRESS_KEY => {
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
                    Some(value.into()),
                    std::mem::take(&mut self.extra),
                )?;

                Ok(Some(event))
            }
            key => {
                if let Some(old_value) = self.extra.insert(key.into(), value.into()) {
                    Err(LineBuilderError::DuplicateKey {
                        key: key.into(),
                        old_value,
                        new_value: value.into(),
                    })
                } else {
                    Ok(None)
                }
            }
        }
    }
}
