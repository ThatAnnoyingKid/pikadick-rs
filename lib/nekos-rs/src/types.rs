use http::uri::InvalidUri;
use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageList {
    pub images: Vec<Image>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ImageUri(pub hyper::Uri);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub artist: Option<String>,
    pub nsfw: bool,
    pub tags: Vec<String>,
    pub likes: u32,
    pub favorites: u32,
    pub uploader: ShortUser,
    pub approver: Option<ShortUser>,
    pub comments: Vec<serde_json::Value>,

    #[serde(rename = "originalHash")]
    pub original_hash: String,

    #[serde(rename = "createdAt")]
    pub created_at: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Image {
    pub fn uri(&self) -> Result<ImageUri, InvalidUri> {
        format!("https://nekos.moe/image/{}", self.id)
            .parse()
            .map(ImageUri)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortUser {
    pub id: String,
    pub username: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
