use std::collections::HashMap;
use url::Url;

/// DeviantArt Search Results
///
#[derive(Debug, serde::Deserialize)]
pub struct SearchResults {
    /// Deviations
    ///
    pub deviations: Vec<Deviation>,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A Deviation
///
#[derive(Debug, serde::Deserialize)]
pub struct Deviation {
    /// DeviantArt Author
    ///
    pub author: serde_json::Value,

    /// ?
    ///
    #[serde(rename = "blockReasons")]
    pub block_reasons: Vec<serde_json::Value>,

    /// Deviation ID
    ///
    #[serde(rename = "deviationId")]
    pub deviation_id: u64,

    /// Deviation Type
    ///
    #[serde(rename = "type")]
    pub kind: String,

    /// Image Url
    ///
    pub url: Url,

    /// Media Info
    ///
    pub media: DeviationMedia,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Deviation {
    /// Get the media url for this [`Deviation`].
    ///
    pub fn get_media_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(0)?);
        Some(url)
    }
}

/// The media field of a [`Deviation`].
///
#[derive(Debug, serde::Deserialize)]
pub struct DeviationMedia {
    /// The base uri
    ///
    #[serde(rename = "baseUri")]
    pub base_uri: Option<Url>,

    /// Image token
    ///
    #[serde(default)]
    pub token: Vec<String>,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// DeviantArt OEmbed
///
#[derive(Debug, serde::Deserialize)]
pub struct OEmbed {
    /// Url of the asset
    ///
    pub url: Url,

    /// Url of the thumbnail
    ///
    pub thumbnail_url: Url,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
