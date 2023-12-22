use super::Md5Digest;
use std::num::NonZeroU64;
use time::OffsetDateTime;
use url::Url;

/// A list of posts
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PostList {
    /// The # of posts.
    ///
    /// This is the total # of posts, not the # in this list.
    #[serde(rename = "@count")]
    pub count: u64,

    /// The current offset
    #[serde(rename = "@offset")]
    pub offset: u64,

    /// The posts
    #[serde(rename = "post", default)]
    pub posts: Box<[Post]>,
}

/// A Post
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Post {
    /// The height of the original file.
    #[serde(rename = "@height")]
    pub height: NonZeroU64,

    /// The number of up-votes.
    #[serde(rename = "@score")]
    pub score: u64,

    /// The main file url
    #[serde(rename = "@file_url")]
    pub file_url: Url,

    /// The parent post id
    #[serde(rename = "@parent_id", with = "serde_optional_str_non_zero_u64")]
    pub parent_id: Option<NonZeroU64>,

    /// The sample url
    #[serde(rename = "@sample_url")]
    pub sample_url: Url,

    /// The sample width
    #[serde(rename = "@sample_width")]
    pub sample_width: NonZeroU64,

    /// The sample height
    #[serde(rename = "@sample_height")]
    pub sample_height: NonZeroU64,

    /// The preview url
    #[serde(rename = "@preview_url")]
    pub preview_url: Url,

    /// The image rating
    #[serde(rename = "@rating")]
    pub rating: Rating,

    /// A list of tag names.
    ///
    /// Tag names are separated by one or more spaces.
    /// There may ore may not be a leading or trailing space.
    /// Tag names are always lowercase.
    #[serde(rename = "@tags")]
    pub tags: Box<str>,

    /// The id the post
    #[serde(rename = "@id")]
    pub id: NonZeroU64,

    /// image width
    #[serde(rename = "@width")]
    pub width: NonZeroU64,

    /// The time of the last change?
    ///
    /// This is a unix timestamp.
    #[serde(rename = "@change", with = "time::serde::timestamp")]
    pub change: OffsetDateTime,

    /// The md5 hash of the file.
    #[serde(rename = "@md5")]
    pub md5: Md5Digest,

    /// The creator id.
    #[serde(rename = "@creator_id")]
    pub creator_id: NonZeroU64,

    /// Whether this has children.
    #[serde(rename = "@has_children")]
    pub has_children: bool,

    /// The creation date of the post.
    #[serde(rename = "@created_at", with = "crate::util::asctime_with_offset")]
    pub created_at: OffsetDateTime,

    /// The status of the post.
    #[serde(rename = "@status")]
    pub status: PostStatus,

    /// The original source.
    ///
    /// May or may not be a url, it is filled manually by users.
    #[serde(rename = "@source", with = "serde_empty_box_str_is_none")]
    pub source: Option<Box<str>>,

    /// Whether the post has notes.
    #[serde(rename = "@has_notes")]
    pub has_notes: bool,

    /// Whether this post has comments.
    #[serde(rename = "@has_comments")]
    pub has_comments: bool,

    /// The preview image width.
    #[serde(rename = "@preview_width")]
    pub preview_width: NonZeroU64,

    /// The preview image height.
    #[serde(rename = "@preview_height")]
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

/// A Post Status
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PostStatus {
    /// Active, the default state.
    #[serde(rename = "active")]
    Active,

    /// Pending, probably waiting for moderator approval.
    #[serde(rename = "pending")]
    Pending,

    /// Deleted, the post has been deleted and metadata will soon be purged.
    #[serde(rename = "deleted")]
    Deleted,

    /// Flagged, the post is has been flagged for review by a moderator.
    #[serde(rename = "flagged")]
    Flagged,
}

mod serde_empty_box_str_is_none {
    use serde::Deserialize;

    pub(super) fn serialize<S>(value: &Option<Box<str>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) if value.is_empty() => serializer.serialize_none(),
            Some(value) => serializer.serialize_str(value),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Box<str>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = Box::<str>::deserialize(deserializer)?;
        if string.is_empty() {
            Ok(None)
        } else {
            Ok(Some(string))
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
