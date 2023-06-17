use bitflags::bitflags;
use std::str::FromStr;

bitflags! {
    /// Encoder capabilities
    #[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
    pub struct Capabilities: u16 {
        const VIDEO = 1 << 1;
        const AUDIO = 1 << 2;
        const SUBTITLE = 1 << 3;
        const FRAME_LEVEL_MULTITHREADING = 1 << 4;
        const SLICE_LEVEL_MULTITHREADING = 1 << 5;
        const EXPERIMENTAL = 1 << 6;
        const DRAW_HORIZ_BAND = 1 << 7;
        const DIRECT_RENDERING_METHOD_1 = 1 << 8;
    }
}

/// An error that may occur while parsing capabilities from a string
#[derive(Debug, thiserror::Error)]
pub enum FromStrError {
    /// Missing a field
    #[error("missing field")]
    Missing,

    /// A field had an invalid value
    #[error("invalid field")]
    Invalid,

    /// A field was found when it was not expected
    #[error("unexpected field")]
    Unexpected,
}

impl FromStr for Capabilities {
    type Err = FromStrError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut flags = Self::empty();
        let mut iter = input.bytes();
        match iter.next().ok_or(FromStrError::Missing)? {
            b'V' => flags.insert(Capabilities::VIDEO),
            b'A' => flags.insert(Capabilities::AUDIO),
            b'S' => flags.insert(Capabilities::SUBTITLE),
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        match iter.next().ok_or(FromStrError::Missing)? {
            b'F' => flags.insert(Capabilities::FRAME_LEVEL_MULTITHREADING),
            b'.' => {}
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        match iter.next().ok_or(FromStrError::Missing)? {
            b'S' => flags.insert(Capabilities::SLICE_LEVEL_MULTITHREADING),
            b'.' => {}
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        match iter.next().ok_or(FromStrError::Missing)? {
            b'X' => flags.insert(Capabilities::EXPERIMENTAL),
            b'.' => {}
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        match iter.next().ok_or(FromStrError::Missing)? {
            b'B' => flags.insert(Capabilities::DRAW_HORIZ_BAND),
            b'.' => {}
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        match iter.next().ok_or(FromStrError::Missing)? {
            b'D' => flags.insert(Capabilities::DIRECT_RENDERING_METHOD_1),
            b'.' => {}
            _ => {
                return Err(FromStrError::Invalid);
            }
        }

        if iter.next().is_some() {
            return Err(FromStrError::Unexpected);
        }

        Ok(flags)
    }
}

/// An error that occurs while parsing an Encoder from a line
#[derive(Debug, thiserror::Error)]
pub enum FromLineError {
    /// Missing the capabilities
    #[error("missing capabilities")]
    MissingCapabilities,

    /// Missing the name
    #[error("missing name")]
    MissingName,

    /// Missing the description
    #[error("missing description")]
    MissingDescription,

    /// Invalid Capabilities
    #[error("invalid capabilities")]
    InvalidCapabilities(#[from] FromStrError),
}

#[derive(Debug)]
pub struct Encoder {
    /// Encoder capabilities
    pub capabilities: Capabilities,

    /// The name
    pub name: Box<str>,

    /// The description
    pub description: Box<str>,
}

impl Encoder {
    /// Parse an encoder from an output line
    pub(crate) fn from_line(line: &str) -> Result<Self, FromLineError> {
        let mut iter = line.splitn(3, char::is_whitespace);
        let capabilities_str = iter.next().ok_or(FromLineError::MissingCapabilities)?;
        let name = iter.next().ok_or(FromLineError::MissingName)?;
        let description = iter.next().ok_or(FromLineError::MissingDescription)?.trim();

        let capabilities = capabilities_str.parse()?;

        Ok(Encoder {
            capabilities,
            name: name.into(),
            description: description.into(),
        })
    }

    /// Returns `true` if this is a video encoder
    pub fn is_video(&self) -> bool {
        self.capabilities.contains(Capabilities::VIDEO)
    }
}
