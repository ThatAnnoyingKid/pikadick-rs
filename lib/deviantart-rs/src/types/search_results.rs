use super::Deviation;
use std::collections::HashMap;

/// DeviantArt Search Results
#[derive(Debug, serde::Deserialize)]
pub struct SearchResults {
    /// The current offset
    #[serde(rename = "currentOffset")]
    pub current_offset: u64,

    /// Deviations
    #[serde(default)]
    pub deviations: Vec<Deviation>,

    /// The Error Code
    #[serde(rename = "errorCode")]
    pub error_code: Option<u32>,

    /// The setimated number of total results
    #[serde(rename = "estTotal")]
    pub est_total: u64,

    /// Whether there are more results
    #[serde(rename = "hasMore")]
    pub has_more: bool,

    /// ?
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const SEARCH_ERROR: &str = include_str!("../../test_data/search_error.json");

    #[test]
    fn parse() {
        let search_results: SearchResults =
            serde_json::from_str(SEARCH_ERROR).expect("failed to parse search results");
        dbg!(search_results);
    }
}
