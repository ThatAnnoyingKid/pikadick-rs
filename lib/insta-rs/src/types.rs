/// The [`MediaInfo`] type
pub mod media_info;
/// The [`PostPage`] type
pub mod post_page;

pub use self::{
    media_info::{
        MediaInfo,
        MediaType,
    },
    post_page::PostPage,
};
use url::Url;

/// The response for a login
#[derive(Debug, serde::Deserialize)]
pub struct LoginResponse {
    /// Whether the user successfully logged in
    pub authenticated: bool,

    /// ?
    pub status: String,

    /// ?
    pub user: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectionListing {
    /// ?
    pub auto_load_more_enabled: bool,

    /// Collection items
    pub items: Vec<Collection>,

    /// ?
    pub more_available: bool,

    /// ?
    pub status: Box<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Collection {
    /// ?
    pub collection_id: Box<str>,

    /// ?
    pub collection_media_count: u32,

    /// ?
    pub collection_name: Box<str>,

    /// ?
    pub collection_type: Box<str>,

    /// ?
    pub cover_media_list: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SavedPostsQueryResult {
    /// The data
    pub data: Data,

    /// The status
    pub status: Box<str>,
}

/// The data field
#[derive(Debug, serde::Deserialize)]
pub struct Data {
    /// ?
    pub user: User,
}

/// The user field
#[derive(Debug, serde::Deserialize)]
pub struct User {
    /// ?
    pub edge_saved_media: EdgeSavedMedia,
}

/// The edge_saved_media field
#[derive(Debug, serde::Deserialize)]
pub struct EdgeSavedMedia {
    /// ?
    pub count: u32,

    /// ?
    pub edges: Vec<Edge>,

    /// Info about the page
    pub page_info: PageInfo,
}

/// An entry in edges
#[derive(Debug, serde::Deserialize)]
pub struct Edge {
    pub node: Node,
}

/// The node field
#[derive(Debug, serde::Deserialize)]
pub struct Node {
    /// caption?
    pub accessibility_caption: Option<Box<str>>,

    /// Whether comments are disabled
    pub comments_disabled: bool,

    /// dimensions?
    pub dimensions: serde_json::Value,

    /// ?
    pub display_url: Url,

    /// ?
    pub edge_liked_by: serde_json::Value,

    /// ?
    pub edge_media_preview_like: serde_json::Value,

    /// ?
    pub edge_media_to_caption: EdgeMediaToCaption,

    /// ?
    pub edge_media_to_comment: serde_json::Value,

    /// media_id?
    pub id: Box<str>,

    /// Whether this is a video
    pub is_video: bool,

    /// Poster?
    pub owner: serde_json::Value,

    /// The shortcode
    pub shortcode: Box<str>,

    /// ?
    pub taken_at_timestamp: u64,

    /// Thumbnail source?
    pub thumbnail_src: Url,

    /// ?
    #[serde(rename = "__typename")]
    pub typename: Box<str>,
}

/// The edge_media_to_caption field
#[derive(Debug, serde::Deserialize)]
pub struct EdgeMediaToCaption {
    pub edges: Vec<EdgeMediaToCaptionEdge>,
}

/// A member of the edges field
#[derive(Debug, serde::Deserialize)]
pub struct EdgeMediaToCaptionEdge {
    pub node: EdgeMediaToCaptionEdgeNode,
}

/// The node field
#[derive(Debug, serde::Deserialize)]
pub struct EdgeMediaToCaptionEdgeNode {
    /// caption?
    pub text: Box<str>,
}

/// The page info
#[derive(Debug, serde::Deserialize)]
pub struct PageInfo {
    /// The end cursor
    pub end_cursor: Box<str>,

    /// Whether this has a next page
    pub has_next_page: bool,
}
