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
pub struct ScrapedWebPageInfo {
    /// ?
    #[serde(rename = "@@config")]
    pub config: Config,

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

impl ScrapedWebPageInfo {
    /// Get the [`Deviation`] for this page.
    pub fn get_current_deviation(&self) -> Option<&Deviation> {
        let id = self.duper_browse.root_stream.as_ref()?.current_open_item;

        let mut buffer = itoa::Buffer::new();
        self.entities.deviation.get(buffer.format(id))
    }
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// The page's csrf token
    #[serde(rename = "csrfToken")]
    pub csrf_token: String,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
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
    pub root_stream: Option<RootStream>,

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

    const SCRAPED_WEBPAGE: &str = include_str!("../test_data/scraped_webpage.json");

    #[test]
    fn parse_scraped_deviation() {
        let scraped_webpage_info: ScrapedWebPageInfo =
            serde_json::from_str(SCRAPED_WEBPAGE).expect("failed to parse scraped webpage info");
        let root_stream = scraped_webpage_info
            .duper_browse
            .root_stream
            .as_ref()
            .expect("missing root stream");

        assert_eq!(root_stream.current_open_item, 119577071);
        // dbg!(scraped_deviation_info.entities.deviation);
    }
}
