/// [`ResultEntry`] types
pub mod result_entry;

pub use self::result_entry::ResultEntry;
use std::collections::HashMap;

/// A JSON search result
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchJson {
    /// Header
    pub header: Header,

    /// Results
    pub results: Vec<ResultEntry>,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Search json result header
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Header {
    /// user id?
    pub user_id: String,
    /// account type?
    pub account_type: String,
    /// Short limit?
    pub short_limit: String,
    /// Long limit?
    pub long_limit: String,
    /// long remaining?
    pub long_remaining: u64,
    /// short remaining
    pub short_remaining: u64,
    /// status?
    pub status: u64,
    /// results requested?
    ///
    /// This may be a string or a number
    pub results_requested: serde_json::Value,
    /// index?
    pub index: HashMap<String, IndexEntry>,
    /// search depth?
    pub search_depth: String,
    /// minimum similarity?
    pub minimum_similarity: f64,
    /// a path to the image maybe?
    pub query_image_display: String,
    /// the query image name
    pub query_image: String,
    /// the number of results returned
    pub results_returned: u64,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// An entry in the header index
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IndexEntry {
    /// status?
    pub status: u64,
    /// parent id?
    pub parent_id: u64,
    /// id?
    pub id: u64,
    /// results?
    pub results: u64,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const SAMPLE: &str = include_str!("../../test_data/sample.json");
    const IMGUR: &str = include_str!("../../test_data/imgur.json");

    #[test]
    fn parse_search_json() {
        let res: SearchJson = serde_json::from_str(SAMPLE).expect("failed to parse");
        dbg!(&res);

        for result in res.results.iter() {
            for extra in result.data.extra.iter() {
                panic!("unknown data: {:#?}", extra);
            }
        }
    }

    #[test]
    fn parse_imgur_json() {
        let res: SearchJson = serde_json::from_str(IMGUR).expect("failed to parse");
        dbg!(&res);

        for result in res.results.iter() {
            for extra in result.data.extra.iter() {
                panic!("unknown data: {:#?}", extra);
            }
        }
    }
}
