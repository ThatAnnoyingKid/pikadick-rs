/// Deviation
pub mod deviation;
/// Search Results
pub mod search_results;

pub use self::{
    deviation::Deviation,
    search_results::SearchResults,
};
use std::collections::HashMap;
use url::Url;

/// DeviantArt OEmbed
#[derive(Debug, serde::Deserialize)]
pub struct OEmbed {
    /// Url of the asset
    pub url: Url,

    /// Url of the thumbnail
    pub thumbnail_url: Option<Url>,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Info scraped from a deviation url
#[derive(Debug, serde::Deserialize)]
pub struct ScrapedDeviationInfo {
    /// ?
    #[serde(rename = "@@entities")]
    pub entities: Entities,

    /// ?
    #[serde(rename = "@@DUPERBROWSE")]
    pub duper_browse: DuperBrowse,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl ScrapedDeviationInfo {
    /// Get the [`Deviation`] for this page.
    pub fn get_current_deviation(&self) -> Option<&Deviation> {
        let id = self.duper_browse.root_stream.current_open_item;

        let mut buffer = itoa::Buffer::new();
        self.entities.deviation.get(buffer.format(id))
    }
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct Entities {
    /// Deviations
    pub deviation: HashMap<String, Deviation>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct DuperBrowse {
    /// ?
    #[serde(rename = "rootStream")]
    pub root_stream: RootStream,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct RootStream {
    /// The id of the current deviation
    #[serde(rename = "currentOpenItem")]
    pub current_open_item: u64,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const SCRAPED_DEVIATION: &str = include_str!("../test_data/scraped_deviation.json");

    #[test]
    fn parse_scraped_deviation() {
        let scraped_deviation_info: ScrapedDeviationInfo = serde_json::from_str(SCRAPED_DEVIATION)
            .expect("failed to parse scraped deviation info");

        assert_eq!(
            scraped_deviation_info
                .duper_browse
                .root_stream
                .current_open_item,
            119577071
        );
        // dbg!(scraped_deviation_info.entities.deviation);
    }
}
