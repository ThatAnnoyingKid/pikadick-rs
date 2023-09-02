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
    pub height: u64,

    /// The number of up-votes.
    #[serde(alias = "@score")]
    pub score: u64,

    /// The main file url
    #[serde(alias = "@file_url")]
    pub file_url: Url,

    /// The parent post id
    #[serde(alias = "@parent_id")]
    pub parent_id: Option<NonZeroU64>,

    /// The sample url
    #[serde(alias = "@sample_url")]
    pub sample_url: Url,

    /// The sample width
    #[serde(alias = "@sample_width")]
    pub sample_width: u64,

    /// The sample height
    #[serde(alias = "@sample_height")]
    pub sample_height: u64,

    /// The preview url
    #[serde(alias = "@preview_url")]
    pub preview_url: Url,

    /// The image rating
    #[serde(alias = "@rating")]
    pub rating: Rating,

    /// Tags
    #[serde(alias = "@tags")]
    pub tags: Box<str>,

    /// The id the post
    #[serde(alias = "@id")]
    pub id: NonZeroU64,

    /// image width
    #[serde(alias = "@width")]
    pub width: u64,

    /// ?
    #[serde(alias = "@change")]
    pub change: u64,

    /// The md5 hash of the file.
    #[serde(alias = "@md5")]
    pub md5: Box<str>,

    /// The creator id.
    #[serde(alias = "@creator_id")]
    pub creator_id: u64,

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
    pub preview_width: u64,

    /// The preview image height.
    #[serde(alias = "@preview_height")]
    pub preview_height: u64,
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
