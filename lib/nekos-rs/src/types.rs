use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashMap;
use url::Url;

/// A list of neko images
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageList {
    /// Images list
    pub images: Vec<Image>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

// Neko images
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// Image id
    pub id: String,
    /// Artist
    pub artist: Option<String>,
    /// Whether this is nsfw
    pub nsfw: bool,
    /// Tags
    pub tags: Vec<String>,
    /// # of likes
    pub likes: u64,
    /// # of favorites
    pub favorites: u64,
    /// The uploader
    pub uploader: ShortUser,
    /// The approver
    pub approver: Option<ShortUser>,
    /// Comments
    pub comments: Vec<serde_json::Value>,

    /// unknown
    #[serde(rename = "originalHash")]
    pub original_hash: String,

    /// created date
    #[serde(rename = "createdAt")]
    pub created_at: String,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Image {
    /// Get the url
    pub fn get_url(&self) -> Result<Url, url::ParseError> {
        let base = Url::parse("https://nekos.moe/image").expect("Valid Base");
        base.join(&self.id)
    }
}

/// A user with only small amounts of info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortUser {
    /// User ID
    pub id: String,
    /// User name
    pub username: String,

    /// Unknown info
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const RANDOM: &str = include_str!("../test_data/random.json");

    #[test]
    fn parse_image_list() {
        let image_list: ImageList = serde_json::from_str(RANDOM).unwrap();
        dbg!(image_list);
    }
}
