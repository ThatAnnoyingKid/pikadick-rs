use crate::obfs::{
    decode::{
        char_decode_default,
        even_odd_char_decode,
        is_char_valid_for_obfs,
    },
    BASE_CHAR_CODE_FOR_LENGTH,
};
use std::borrow::Cow;

pub trait Options {
    /// Called on the obfuscated str before the deob process.
    fn modify_str<'a>(&self, ostr: &'a str, _key: &str, _key_sum: u32) -> Option<Cow<'a, str>> {
        Some(ostr.into())
    }

    fn extract_key_sum(&self, key: &str, _ostr: &str) -> Option<u32> {
        Some(u32::from(key.chars().next()?))
    }

    /// Function that "deobfuscates" the data
    fn decode_char(&self, c: char, offset: i64, _index: usize, _version: u32) -> Option<char> {
        char_decode_default(c, offset)
    }
}

pub struct DefaultOptions;
impl Options for DefaultOptions {}

pub struct StringOptions;
impl Options for StringOptions {
    fn modify_str<'a>(&self, ostr: &'a str, _key: &str, _key_sum: u32) -> Option<Cow<'a, str>> {
        let target_char = *ostr.as_bytes().iter().nth(ostr.len() - 2)?;
        let key_len = target_char as usize - BASE_CHAR_CODE_FOR_LENGTH;
        let start_byte_pos = ostr.char_indices().nth(key_len)?.0;
        let ret = ostr.get(start_byte_pos..ostr.len() - 2)?;

        Some(ret.into())
    }

    fn extract_key_sum(&self, key: &str, _ostr: &str) -> Option<u32> {
        let first = u32::from(key.chars().next()?);
        let last = u32::from(key.chars().nth(key.len() - 1)?);
        Some(first + last)
    }

    fn decode_char(&self, c: char, offset: i64, index: usize, version: u32) -> Option<char> {
        match version {
            2 => {
                if is_char_valid_for_obfs(c) {
                    even_odd_char_decode(c, offset, index)
                } else {
                    Some(c)
                }
            }
            1 => even_odd_char_decode(c, offset, index),
            _ => even_odd_char_decode(c, offset, index),
        }
    }
}
