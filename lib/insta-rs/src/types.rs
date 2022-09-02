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
