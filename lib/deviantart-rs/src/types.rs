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
    /// Page config like csrf tokens
    #[serde(rename = "@@config")]
    pub config: Config,

    /// Deviations extended deviations maybe?
    #[serde(rename = "@@entities")]
    pub entities: Option<Entities>,

    /// ?
    #[serde(rename = "@@DUPERBROWSE")]
    pub duper_browse: Option<DuperBrowse>,

    /// Info about the current session
    #[serde(rename = "@@publicSession")]
    pub public_session: PublicSession,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl ScrapedWebPageInfo {
    /// Get the current deviation's id
    pub fn get_current_deviation_id(&self) -> Option<u64> {
        Some(
            self.duper_browse
                .as_ref()?
                .root_stream
                .as_ref()?
                .current_open_item,
        )
    }

    /// Get the [`Deviation`] for this page.
    pub fn get_current_deviation(&self) -> Option<&Deviation> {
        let id = self.get_current_deviation_id()?;
        self.entities.as_ref()?.deviation.get(&id)
    }

    /// Get the [`DeviationExtended`] for this page.
    pub fn get_current_deviation_extended(&self) -> Option<&DeviationExtended> {
        let id = self.get_current_deviation_id()?;
        self.entities
            .as_ref()?
            .deviation_extended
            .as_ref()?
            .get(&id)
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
    pub deviation: HashMap<u64, Deviation>,

    /// Extended Deviation Info
    #[serde(rename = "deviationExtended")]
    pub deviation_extended: Option<HashMap<u64, DeviationExtended>>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Extended Info about a deviation
#[derive(Debug, serde::Deserialize)]
pub struct DeviationExtended {
    /// Download info
    pub download: Option<Download>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Download {
    /// The file size
    pub filesize: u64,

    /// The image height
    pub height: u32,

    /// The image width
    pub width: u32,

    /// ?
    #[serde(rename = "type")]
    pub kind: String,

    /// The url
    pub url: Url,

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

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct PublicSession {
    /// Whether the user is logged in
    #[serde(rename = "isLoggedIn")]
    pub is_logged_in: bool,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const SCRAPED_WEBPAGE: &str = include_str!("../test_data/scraped_webpage.json");
    const LOGIN_WEBPAGE: &str = include_str!("../test_data/login_webpage.json");

    #[test]
    fn parse_scraped_webpage() {
        let scraped_webpage_info: ScrapedWebPageInfo =
            serde_json::from_str(SCRAPED_WEBPAGE).expect("failed to parse scraped webpage info");
        assert_eq!(
            scraped_webpage_info
                .get_current_deviation_id()
                .expect("missing current deviation id"),
            119577071
        );
        // dbg!(scraped_deviation_info.entities.deviation);
    }

    #[test]
    fn parse_login_webpage() {
        let _scraped_webpage_info: ScrapedWebPageInfo =
            serde_json::from_str(LOGIN_WEBPAGE).expect("failed to parse scraped webpage info");
    }
}
