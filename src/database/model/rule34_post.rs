use super::JiffTimestampWrapper;
use nd_async_rusqlite::rusqlite::{
    Result as RusqliteResult,
    Row as RusqliteRow,
};
use std::num::NonZeroU64;

/// A rule34 post.
#[derive(Debug, Clone)]
pub struct Rule34Post {
    pub id: NonZeroU64,
    pub tags: String,
    pub file_url: String,
    pub last_fetched: jiff::Timestamp,
}

impl Rule34Post {
    pub(crate) fn from_row(row: &RusqliteRow<'_>) -> RusqliteResult<Self> {
        let id = row.get("id")?;
        let tags = row.get("tags")?;
        let file_url = row.get("file_url")?;
        let last_fetched: JiffTimestampWrapper = row.get("last_fetched")?;

        Ok(Self {
            id,
            tags,
            file_url,
            last_fetched: last_fetched.0,
        })
    }
}
