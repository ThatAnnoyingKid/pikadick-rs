use std::collections::HashMap;
use url::Url;

/// A u8 was not a valid media type
#[derive(Debug)]
pub struct InvalidMediaTypeError(u8);

impl std::fmt::Display for InvalidMediaTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` is not a valid media type", self.0)
    }
}

impl std::error::Error for InvalidMediaTypeError {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Deserialize)]
#[serde(try_from = "u8")]
pub enum MediaType {
    /// A Photo
    Photo,

    /// A video
    Video,

    /// A carousel
    Carousel,
}

impl TryFrom<u8> for MediaType {
    type Error = InvalidMediaTypeError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            1 => Ok(Self::Photo),
            2 => Ok(Self::Video),
            8 => Ok(Self::Carousel),
            _ => Err(InvalidMediaTypeError(n)),
        }
    }
}

/// AdditionalDataLoaded
#[derive(Debug, serde::Deserialize)]
pub struct AdditionalDataLoaded {
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
    pub media_type: MediaType,

    /// Versions of a video post.
    ///
    /// Only present on video posts
    pub video_versions: Option<Vec<VideoVersion>>,

    /// Versions of an image post
    pub image_versions2: Option<ImageVersions2>,

    /// Carousel media
    pub carousel_media: Option<Vec<CarouselMediaItem>>,

    /// The post code
    pub code: String,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl AdditionalDataLoadedItem {
    /// Get the best image_versions2 candidate
    pub fn get_best_image_versions2_candidate(&self) -> Option<&ImageVersions2Candidate> {
        self.image_versions2.as_ref()?.get_best()
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

impl ImageVersions2 {
    /// Get the best candidate
    pub fn get_best(&self) -> Option<&ImageVersions2Candidate> {
        self.candidates
            .iter()
            .max_by_key(|image_versions2_candidate| image_versions2_candidate.height)
    }
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

/// An item in carousel_media
#[derive(Debug, serde::Deserialize)]
pub struct CarouselMediaItem {
    /// The media type
    pub media_type: MediaType,

    /// Image versions
    pub image_versions2: Option<ImageVersions2>,

    /// Extra fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl CarouselMediaItem {
    /// Get the best image_versions2 candidate
    pub fn get_best_image_versions2_candidate(&self) -> Option<&ImageVersions2Candidate> {
        self.image_versions2.as_ref()?.get_best()
    }
}
