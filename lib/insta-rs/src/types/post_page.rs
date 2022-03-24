use std::collections::HashMap;
use url::Url;

/// A Post Page
#[derive(Debug, serde::Deserialize)]
pub struct PostPage {
    /// ?
    pub num_results: u32,

    /// ?
    pub items: Vec<AdditionalDataLoadedItem>,

    /// ?
    pub auto_load_more_enabled: bool,

    /// ?
    pub more_available: bool,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdditionalDataLoadedItem {
    /// ?
    pub video_versions: Vec<VideoVersion>,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl AdditionalDataLoadedItem {
    /// Get the best video version
    pub fn get_best_video_version(&self) -> Option<&VideoVersion> {
        self.video_versions
            .iter()
            .max_by_key(|video_version| video_version.height)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct VideoVersion {
    /// The height in pixels
    pub height: u32,

    /// The width in pixels
    pub width: u32,

    /// ?
    #[serde(rename = "type")]
    pub kind: u32,

    /// the src url
    pub url: Url,

    /// ?
    pub id: String,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
