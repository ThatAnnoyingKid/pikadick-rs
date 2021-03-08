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

    /// Get the download url for this [`Deviation`].
    ///
    pub fn get_download_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(1)?);
        Some(url)
    }

    /// Get the fullview url for this [`Deviation`].
    ///
    pub fn get_fullview_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();
        url.path_segments_mut()
            .ok()?
            .push(&self.media.get_fullview_media_type()?.content.as_ref()?);
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(0)?);
        Some(url)
    }

    /// Whether this is an image
    ///
    pub fn is_image(&self) -> bool {
        self.kind == "image"
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

    /// Types
    ///
    pub types: Vec<MediaType>,

    /// Pretty Name
    ///
    #[serde(rename = "prettyName")]
    pub pretty_name: Option<String>,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl DeviationMedia {
    /// Try to get the fullview [`MediaType`].
    ///
    pub fn get_fullview_media_type(&self) -> Option<&MediaType> {
        self.types.iter().find(|t| t.is_fullview())
    }
}

/// DeviantArt [`DeviationMedia`] media type.
///
#[derive(Debug, serde::Deserialize)]
pub struct MediaType {
    /// The content. A uri used with base_uri.
    ///
    #[serde(rename = "c")]
    pub content: Option<String>,

    /// Image Height
    ///
    #[serde(rename = "h")]
    pub height: u64,

    /// ?
    ///
    pub r: u64,

    /// The kind of media
    ///
    #[serde(rename = "t")]
    pub kind: String,

    /// Image Width
    ///
    #[serde(rename = "w")]
    pub width: u64,

    /// ?
    ///
    pub f: Option<u64>,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl MediaType {
    /// Whether this is the fullview
    ///
    pub fn is_fullview(&self) -> bool {
        self.kind == "fullview"
    }
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
    pub thumbnail_url: Option<Url>,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
