/// Deviation
pub mod deviation;
/// Scraped stash info
pub mod scraped_stash_info;
/// Scraped webpage info
pub mod scraped_webpage_info;
/// Search Results
pub mod search_results;

pub use self::{
    deviation::Deviation,
    scraped_stash_info::ScrapedStashInfo,
    scraped_webpage_info::{
        DeviationExtended,
        ScrapedWebPageInfo,
    },
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

    /// Title
    pub title: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
