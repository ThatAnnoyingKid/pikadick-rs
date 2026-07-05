use super::JiffTimestampWrapper;
use nd_async_rusqlite::rusqlite::{
    Result as RusqliteResult,
    Row as RusqliteRow,
};

/// A rule34 query stat.
#[derive(Debug)]
pub struct Rule34QueryStat {
    pub query: String,
    pub queries_since_last_fetch: u32,
    pub last_fetched: jiff::Timestamp,
}

impl Rule34QueryStat {
    pub(crate) fn from_row(row: &RusqliteRow<'_>) -> RusqliteResult<Self> {
        let query = row.get("query")?;
        let queries_since_last_fetch = row.get("queries_since_last_fetch")?;
        let last_fetched: JiffTimestampWrapper = row.get("last_fetched")?;

        Ok(Self {
            query,
            queries_since_last_fetch,
            last_fetched: last_fetched.0,
        })
    }
}
