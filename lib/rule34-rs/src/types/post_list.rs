use super::Md5Digest;
use std::num::NonZeroU64;
use time::{
    format_description::FormatItem,
    OffsetDateTime,
};
use url::Url;

const ASCTIME_WITH_OFFSET_FORMAT: &[FormatItem<'_>] = time::macros::format_description!(
    "[weekday repr:short] [month repr:short] [day] [hour]:[minute]:[second] [offset_hour][offset_minute] [year]"
);

time::serde::format_description!(
    asctime_with_offset,
    OffsetDateTime,
    ASCTIME_WITH_OFFSET_FORMAT
);

/// A list of posts
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PostList {
    /// The # of posts.
    ///
    /// This is the total # of posts, not the # in this list.
    #[serde(alias = "@count")]
    pub count: u64,

    /// The current offset
    #[serde(alias = "@offset")]
    pub offset: u64,

    /// The posts
    #[serde(alias = "post", default)]
    pub posts: Vec<Post>,
}

/// A Post
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Post {
    /// The height of the original file.
    #[serde(alias = "@height")]
    pub height: NonZeroU64,

    /// The number of up-votes.
    #[serde(alias = "@score")]
    pub score: u64,

    /// The main file url
    #[serde(alias = "@file_url")]
    pub file_url: Url,

    /// The parent post id
    #[serde(alias = "@parent_id", with = "serde_optional_str_non_zero_u64")]
    pub parent_id: Option<NonZeroU64>,

    /// The sample url
    #[serde(alias = "@sample_url")]
    pub sample_url: Url,

    /// The sample width
    #[serde(alias = "@sample_width")]
    pub sample_width: NonZeroU64,

    /// The sample height
    #[serde(alias = "@sample_height")]
    pub sample_height: NonZeroU64,

    /// The preview url
    #[serde(alias = "@preview_url")]
    pub preview_url: Url,

    /// The image rating
    #[serde(alias = "@rating")]
    pub rating: Rating,

    /// A list of tag names.
    ///
    /// Tag names are separated by one or more spaces.
    /// There may ore may not be a leading or trailing space.
    /// Tag names are always lowercase.
    #[serde(alias = "@tags")]
    pub tags: Box<str>,

    /// The id the post
    #[serde(alias = "@id")]
    pub id: NonZeroU64,

    /// image width
    #[serde(alias = "@width")]
    pub width: NonZeroU64,

    /// The time of the last change?
    ///
    /// This is a unix timestamp.
    #[serde(alias = "@change", with = "time::serde::timestamp")]
    pub change: OffsetDateTime,

    /// The md5 hash of the file.
    #[serde(alias = "@md5")]
    pub md5: Md5Digest,

    /// The creator id.
    #[serde(alias = "@creator_id")]
    pub creator_id: NonZeroU64,

    /// Whether this has children.
    #[serde(alias = "@has_children")]
    pub has_children: bool,

    /// The creation date of the post.
    #[serde(alias = "@created_at", with = "asctime_with_offset")]
    pub created_at: OffsetDateTime,

    /// ?
    #[serde(alias = "@status")]
    pub status: Box<str>,

    /// The original source.
    ///
    /// May or may not be a url, it is filled manually by users.
    #[serde(alias = "@source")]
    pub source: Option<Box<str>>,

    /// Whether the post has notes.
    #[serde(alias = "@has_notes")]
    pub has_notes: bool,

    /// Whether this post has comments.
    #[serde(alias = "@has_comments")]
    pub has_comments: bool,

    /// The preview image width.
    #[serde(alias = "@preview_width")]
    pub preview_width: NonZeroU64,

    /// The preview image height.
    #[serde(alias = "@preview_height")]
    pub preview_height: NonZeroU64,
}

impl Post {
    /// Get the html post url for this.
    ///
    /// This allocates, so cache the result.
    pub fn get_html_post_url(&self) -> Url {
        crate::post_id_to_html_post_url(self.id)
    }
}

/// A post rating
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Rating {
    /// Questionable
    #[serde(rename = "q")]
    Questionable,

    /// Explicit
    #[serde(rename = "e")]
    Explicit,

    /// Safe
    #[serde(rename = "s")]
    Safe,
}

impl Rating {
    /// Get this as a char
    pub fn as_char(self) -> char {
        match self {
            Self::Questionable => 'q',
            Self::Explicit => 'e',
            Self::Safe => 's',
        }
    }

    /// Get this as a &str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Questionable => "q",
            Self::Explicit => "e",
            Self::Safe => "s",
        }
    }
}

mod serde_optional_str_non_zero_u64 {
    use serde::de::Error;
    use std::{
        borrow::Cow,
        num::NonZeroU64,
        str::FromStr,
    };

    pub(super) fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let data: Cow<'_, str> = serde::Deserialize::deserialize(deserializer)?;
        if data.is_empty() {
            return Ok(None);
        }

        Ok(Some(data.parse().map_err(D::Error::custom)?))
    }

    pub(super) fn serialize<S>(value: &Option<NonZeroU64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) => {
                let mut buffer = itoa::Buffer::new();
                serializer.serialize_str(buffer.format(value.get()))
            }
            None => serializer.serialize_str(""),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn asctime_with_offset_sanity() {
        let date_time_str = "Sat Sep 02 02:01:00 +0000 2023";
        let date = OffsetDateTime::parse(date_time_str, ASCTIME_WITH_OFFSET_FORMAT)
            .expect("failed to parse");

        assert!(date.unix_timestamp() == 1693620060);
    }
}
