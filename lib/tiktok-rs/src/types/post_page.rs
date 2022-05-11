use once_cell::sync::Lazy;
use scraper::{
    Html,
    Selector,
};
use std::collections::HashMap;
use url::Url;

static SIGI_PERSISTED_DATA_SCRIPT_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("#SIGI_STATE").expect("invalid SIGI_PERSISTED_DATA_SCRIPT_SELECTOR")
});

/// An error that may occur while parsing html
#[derive(thiserror::Error, Debug)]
pub enum FromHtmlError {
    #[error("missing sigi state element")]
    MissingSigiStateElement,

    #[error("missing sigi state")]
    MissingSigiState,

    #[error("invalid sigi state")]
    InvalidSigiState(#[source] serde_json::Error),
}

/// A post page
#[derive(Debug)]
pub struct PostPage {
    pub sigi_state: SigiState,
}

impl PostPage {
    /// Parse from html
    pub(crate) fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let sigi_state_script_str = html
            .select(&SIGI_PERSISTED_DATA_SCRIPT_SELECTOR)
            .next()
            .and_then(|el| el.text().next())
            .ok_or(FromHtmlError::MissingSigiStateElement)?;

        let sigi_state: SigiState =
            serde_json::from_str(sigi_state_script_str).map_err(FromHtmlError::InvalidSigiState)?;

        Ok(Self { sigi_state })
    }

    /// Get the item module post for this post page
    pub fn get_item_module_post(&self) -> Option<&ItemModulePost> {
        self.sigi_state.item_module.posts.values().next()
    }

    /// Get the video download url for a post by id, if it exists
    pub fn get_video_download_url(&self) -> Option<&Url> {
        Some(&self.get_item_module_post()?.video.download_addr)
    }
}

/// Sigi state
#[derive(Debug, serde::Deserialize)]
pub struct SigiState {
    /// ?
    #[serde(rename = "AppContext")]
    pub app_context: serde_json::Value,

    /// ?
    #[serde(rename = "ItemModule")]
    pub item_module: ItemModule,

    /// Unknown k/vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct ItemModule {
    /// Posts
    #[serde(flatten)]
    pub posts: HashMap<String, ItemModulePost>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct ItemModulePost {
    /// Post author
    pub author: String,

    /// Video description
    pub desc: String,

    /// Nickname?
    pub nickname: String,

    /// Stats
    pub stats: serde_json::Value,

    /// Video data
    pub video: ItemModulePostVideo,

    /// Unknown k/vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct ItemModulePostVideo {
    /// Bitrate
    pub bitrate: u32,

    /// Video codec type
    #[serde(rename = "codecType")]
    pub codec_type: String,

    /// a url?
    pub cover: Url,

    /// video definition?
    pub definition: String,

    /// The download address?
    #[serde(rename = "downloadAddr")]
    pub download_addr: Url,

    /// video duration, in seconds
    pub duration: u64,

    /// The video ID
    pub id: String,

    /// The video quality?
    #[serde(rename = "videoQuality")]
    pub video_quality: String,

    /// main url?
    #[serde(rename = "playAddr")]
    pub play_addr: Url,

    /// Height
    pub height: u64,

    /// Width
    pub width: u64,

    /// Video ratio
    pub ratio: String,

    /// Video format
    pub format: String,

    /// ?
    #[serde(rename = "shareCover")]
    pub share_cover: Vec<serde_json::Value>,

    /// ?
    #[serde(rename = "originCover")]
    pub origin_cover: Url,

    /// ?
    #[serde(rename = "encodedType")]
    pub encoded_type: String,

    /// ?
    #[serde(rename = "reflowCover")]
    pub reflow_cover: Url,

    /// ?
    #[serde(rename = "dynamicCover")]
    pub dynamic_cover: Url,

    /// ?
    #[serde(rename = "encodeUserTag")]
    pub encode_user_tag: String,

    /// Unknown k/vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
