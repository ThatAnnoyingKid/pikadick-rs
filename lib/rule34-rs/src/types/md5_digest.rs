use std::borrow::Cow;

const DIGEST_LEN: usize = 16;

/// An error that may occur while parsing an Md5Digest from a &str.
#[derive(Debug, thiserror::Error)]
pub enum FromStrError {
    /// The digest length was invalid.
    #[error("invalid digest length {len}")]
    InvalidLength { len: usize },

    /// There was an invalid hex byte
    #[error("invalid hex byte 0x{byte:X} at position {position}")]
    InvalidHexByte { byte: u8, position: usize },
}

/// An md5 Digest.
///
/// Smaller and faster than a String, and performs basic validation.
#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "Cow<str>", into = "String")]
pub struct Md5Digest(pub [u8; DIGEST_LEN]);

impl TryFrom<&str> for Md5Digest {
    type Error = FromStrError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let input_len = input.len();

        if input_len != DIGEST_LEN * 2 {
            return Err(FromStrError::InvalidLength { len: input_len });
        }

        let mut digest = [0; DIGEST_LEN];
        for (i, (digest_byte, hex)) in digest
            .iter_mut()
            .zip(input.as_bytes().chunks(2))
            .enumerate()
        {
            let high = decode_hex_u8(hex[0]).map_err(|byte| FromStrError::InvalidHexByte {
                byte,
                position: i * 2,
            })?;
            let low = decode_hex_u8(hex[1]).map_err(|byte| FromStrError::InvalidHexByte {
                byte,
                position: (i * 2) + 1,
            })?;

            *digest_byte = high << 4 | low;
        }

        Ok(Self(digest))
    }
}

impl TryFrom<Cow<'_, str>> for Md5Digest {
    type Error = FromStrError;

    fn try_from(input: Cow<str>) -> Result<Self, Self::Error> {
        let input: &str = &input;
        input.try_into()
    }
}

impl From<Md5Digest> for String {
    fn from(digest: Md5Digest) -> Self {
        digest.to_string()
    }
}

impl std::str::FromStr for Md5Digest {
    type Err = FromStrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::try_from(input)
    }
}

impl std::fmt::Display for Md5Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Decode a hex char
fn decode_hex_u8(hex: u8) -> Result<u8, u8> {
    match hex {
        b'A'..=b'F' => Ok(hex - b'A' + 10),
        b'a'..=b'f' => Ok(hex - b'a' + 10),
        b'0'..=b'9' => Ok(hex - b'0'),
        hex => Err(hex),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let digest_str = "d41d8cd98f00b204e9800998ecf8427e";
        let digest = Md5Digest::try_from(digest_str).expect("invalid md5 digest");
        assert!(digest.to_string() == digest_str);
    }
}
