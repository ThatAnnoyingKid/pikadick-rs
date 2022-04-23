use std::convert::TryFrom;

/// Error when a u32 cannot be converted into a platform.
#[derive(Debug)]
pub struct InvalidPlatformCode(pub u32);

impl std::error::Error for InvalidPlatformCode {}

impl std::fmt::Display for InvalidPlatformCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "the code {} is not a valid platform", self.0)
    }
}

/// The representation of a platform
#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "u32")]
#[serde(into = "u32")]
pub enum Platform {
    Pc,
    Xbox,
    Ps4,
}

impl Platform {
    /// Converts a platform into its code
    pub fn as_u32(self) -> u32 {
        match self {
            Platform::Pc => 4,
            Platform::Xbox => 1,
            Platform::Ps4 => 2,
        }
    }

    /// Tries to convert a u32 into a Platform
    pub fn from_u32(n: u32) -> Result<Self, InvalidPlatformCode> {
        match n {
            4 => Ok(Platform::Pc),
            1 => Ok(Platform::Xbox),
            2 => Ok(Platform::Ps4),
            n => Err(InvalidPlatformCode(n)),
        }
    }
}

impl TryFrom<u32> for Platform {
    type Error = InvalidPlatformCode;

    fn try_from(n: u32) -> Result<Self, Self::Error> {
        Self::from_u32(n)
    }
}

impl From<Platform> for u32 {
    fn from(platform: Platform) -> Self {
        platform.as_u32()
    }
}
