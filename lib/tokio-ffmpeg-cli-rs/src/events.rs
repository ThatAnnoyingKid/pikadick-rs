use crate::Error;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ProgressEvent {
    /// The frame number
    pub frame: u64,

    /// The fps
    pub fps: f64,

    /// The bitrate
    pub bitrate: String,

    /// The progress
    pub progress: String,

    /// The total size
    pub total_size: u64,

    /// The out time in us
    pub out_time_us: u64,

    /// The out time in ms
    pub out_time_ms: u64,

    /// The out time
    pub out_time: String,

    /// The # of dup frames
    pub dup_frames: u64,

    /// The # of dropped frames
    pub drop_frames: u64,

    /// The speed.
    /// None means N/A.
    pub speed: Option<f64>,

    /// Extra K/Vs
    pub extra: HashMap<String, String>,
}

impl ProgressEvent {
    /// Try to make a [`ProgressEvent`] from optional parts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_from_optional_parts(
        frame: Option<u64>,
        fps: Option<f64>,
        bitrate: Option<String>,
        total_size: Option<u64>,
        out_time_us: Option<u64>,
        out_time_ms: Option<u64>,
        out_time: Option<String>,
        dup_frames: Option<u64>,
        drop_frames: Option<u64>,
        speed: Option<Option<f64>>,
        progress: Option<String>,
        extra: HashMap<String, String>,
    ) -> Result<Self, Error> {
        Ok(ProgressEvent {
            frame: frame.ok_or(Error::MissingKeyValuePair("frame"))?,
            fps: fps.ok_or(Error::MissingKeyValuePair("fps"))?,
            bitrate: bitrate.ok_or(Error::MissingKeyValuePair("bitrate"))?,
            total_size: total_size.ok_or(Error::MissingKeyValuePair("total_size"))?,
            out_time_us: out_time_us.ok_or(Error::MissingKeyValuePair("out_time_us"))?,
            out_time_ms: out_time_ms.ok_or(Error::MissingKeyValuePair("out_time_ms"))?,
            out_time: out_time.ok_or(Error::MissingKeyValuePair("out_time"))?,
            dup_frames: dup_frames.ok_or(Error::MissingKeyValuePair("dup_frames"))?,
            drop_frames: drop_frames.ok_or(Error::MissingKeyValuePair("drop_frames"))?,
            speed: speed.ok_or(Error::MissingKeyValuePair("speed"))?,
            progress: progress.ok_or(Error::MissingKeyValuePair("progress"))?,
            extra,
        })
    }
}

/// A builder to assemble lines into a [`ProgressEvent`].
#[derive(Debug)]
pub(crate) struct ProgressEventLineBuilder {
    maybe_frame: Option<u64>,
    maybe_fps: Option<f64>,
    maybe_bitrate: Option<String>,
    maybe_total_size: Option<u64>,
    maybe_out_time_us: Option<u64>,
    maybe_out_time_ms: Option<u64>,
    maybe_out_time: Option<String>,
    maybe_dup_frames: Option<u64>,
    maybe_drop_frames: Option<u64>,
    maybe_speed: Option<Option<f64>>,

    extra: HashMap<String, String>,
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

        let mut iter = line.split('=');
        let key = iter.next();
        let value = iter.next();
        if iter.next().is_some() {
            return Err(Error::InvalidKeyValuePair);
        }

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
            (Some("frame"), Some(value)) => {
                parse_kv_u64("frame", value, &mut self.maybe_frame)?;
                Ok(None)
            }
            (Some("fps"), Some(value)) => {
                parse_kv_f64("fps", value, &mut self.maybe_fps)?;
                Ok(None)
            }
            (Some("bitrate"), Some(value)) => {
                let value = value.trim();

                if self.maybe_bitrate.is_some() {
                    Err(Error::DuplicateKey("bitrate".into()))
                } else {
                    self.maybe_bitrate = Some(value.to_string());
                    Ok(None)
                }
            }
            (Some("total_size"), Some(value)) => {
                parse_kv_u64("total_size", value, &mut self.maybe_total_size)?;
                Ok(None)
            }
            (Some("out_time_us"), Some(value)) => {
                parse_kv_u64("out_time_us", value, &mut self.maybe_out_time_us)?;
                Ok(None)
            }
            (Some("out_time_ms"), Some(value)) => {
                parse_kv_u64("out_time_ms", value, &mut self.maybe_out_time_ms)?;
                Ok(None)
            }
            (Some("out_time"), Some(out_time)) => {
                if self.maybe_out_time.is_some() {
                    Err(Error::DuplicateKey("out_time".into()))
                } else {
                    self.maybe_out_time = Some(out_time.to_string());
                    Ok(None)
                }
            }
            (Some("dup_frames"), Some(value)) => {
                parse_kv_u64("dup_frames", value, &mut self.maybe_dup_frames)?;
                Ok(None)
            }
            (Some("drop_frames"), Some(value)) => {
                parse_kv_u64("drop_frames", value, &mut self.maybe_drop_frames)?;
                Ok(None)
            }
            (Some("speed"), Some(value)) => {
                let key = "speed";
                let value = value.trim_end_matches('x').trim_end_matches('X').trim();

                if value == "N/A" {
                    self.maybe_speed = Some(None);
                } else {
                    if self.maybe_speed.is_some() {
                        return Err(Error::DuplicateKey(key.into()));
                    }
                    self.maybe_speed = Some(Some(
                        value
                            .parse()
                            .map_err(|e| Error::InvalidFloatValue(key, e))?,
                    ));
                }

                Ok(None)
            }
            (Some("progress"), Some(progress)) => {
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
                    Some(progress.to_string()),
                    std::mem::take(&mut self.extra),
                )?;

                Ok(Some(event))
            }
            (Some(key), Some(value)) => {
                if self
                    .extra
                    .insert(key.to_string(), value.to_string())
                    .is_some()
                {
                    Err(Error::DuplicateKey(key.to_string()))
                } else {
                    Ok(None)
                }
            }
            (None, _) | (_, None) => Err(Error::InvalidKeyValuePair),
        }
    }
}
