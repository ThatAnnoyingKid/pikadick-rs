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
    /// The media type
    pub media_type: u8,

    /// Versions of a video post.
    ///
    /// Only present on video posts
    pub video_versions: Option<Vec<VideoVersion>>,

    /// Versions of an image post
    pub image_versions2: Option<ImageVersions2>,

    /// The post code
    pub code: String,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl AdditionalDataLoadedItem {
    /// Returns `true` if this is a photo.
    pub fn is_photo(&self) -> bool {
        self.media_type == 1
    }

    /// Returns `true` if this is a video.
    pub fn is_video(&self) -> bool {
        self.media_type == 2
    }

    /// Get the best image_versions2 candidate
    pub fn get_best_image_versions2_candidate(&self) -> Option<&ImageVersions2Candidate> {
        self.image_versions2
            .as_ref()?
            .candidates
            .iter()
            .max_by_key(|image_versions2_candidate| image_versions2_candidate.height)
    }

    /// Get the best video version
    pub fn get_best_video_version(&self) -> Option<&VideoVersion> {
        self.video_versions
            .as_ref()?
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

/// The image_versions2 field
#[derive(Debug, serde::Deserialize)]
pub struct ImageVersions2 {
    /// Candidate images
    pub candidates: Vec<ImageVersions2Candidate>,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A ImageVersions2 candidate
#[derive(Debug, serde::Deserialize)]
pub struct ImageVersions2Candidate {
    /// The image height in pixels
    pub width: u32,

    /// The image width in pixels
    pub height: u32,

    /// The url
    pub url: Url,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
