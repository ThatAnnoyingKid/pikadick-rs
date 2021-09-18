use std::collections::HashMap;
use url::Url;

/// A result for a search
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
    /// ?
    pub change: u64,

    /// ?
    pub directory: u64,

    /// image url
    pub file_url: Url,

    /// image hash
    pub hash: String,

    /// image height
    pub height: u64,

    /// id
    pub id: u64,

    /// image name
    pub image: String,

    /// owner
    pub owner: String,

    /// parent post id?
    pub parent_id: u64,

    /// Preview image url
    pub preview_url: Url,

    /// rating
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

impl SearchResult {
    /// Get the post url for this search entry.
    ///
    /// This allocates, so cache the result.
    pub fn get_post_url(&self) -> Url {
        crate::post_id_to_post_url(self.id)
    }
}

/// Search Result Entry
#[derive(Debug)]
pub struct SearchEntry {
    /// The thumbnail url
    pub thumbnail: Url,

    /// The description
    pub description: String,
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_JSON_STR: &str = include_str!("../../test_data/gif_search.json");

    #[test]
    fn from_gif_json() {
        let results: Vec<SearchResult> =
            serde_json::from_str(GIF_JSON_STR).expect("invalid gif search result");
        dbg!(results);
    }
}
