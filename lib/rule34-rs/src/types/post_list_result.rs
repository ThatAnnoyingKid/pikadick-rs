use std::collections::HashMap;
use url::Url;

/// A result for a list api call
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PostListResult {
    /// ?
    pub change: u64,

    /// ?
    pub directory: u64,

    /// the image url
    pub file_url: Url,

    /// the image hash
    pub hash: String,

    /// the image height
    pub height: u64,

    /// the post id
    pub id: u64,

    /// the image name
    pub image: String,

    /// owner
    pub owner: String,

    /// the parent post id
    pub parent_id: Option<u64>,

    /// Preview image url
    pub preview_url: Url,

    /// the image rating
    pub rating: String,

    /// ?
    pub sample: u64,

    /// ?
    pub sample_height: u64,

    /// ?
    pub sample_url: Url,

    /// ?
    pub sample_width: u64,

    /// ?
    pub score: u64,

    /// Post tags
    ///
    /// This is a string where each tag is seperated by a space character.
    pub tags: String,

    /// image width
    pub width: u64,

    /// Unknown extra values
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl PostListResult {
    /// Get the post url for this list result.
    ///
    /// This allocates, so cache the result.
    pub fn get_post_url(&self) -> Url {
        crate::post_id_to_post_url(self.id)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_JSON_STR: &str = include_str!("../../test_data/gif_list.json");

    #[test]
    fn from_gif_json() {
        let results: Vec<PostListResult> =
            serde_json::from_str(GIF_JSON_STR).expect("invalid gif list result");
        dbg!(results);
    }
}
