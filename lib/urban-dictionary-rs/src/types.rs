use std::collections::HashMap;
use url::Url;

/// A List of [`Definition`].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DefinitionList {
    /// The inner list
    pub list: Vec<Definition>,

    /// Unknown k/vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A [`Definition`] for a term.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Definition {
    /// The author
    pub author: String,

    /// The current votes for this
    pub current_vote: String,

    /// The definition id
    pub defid: u64,

    /// The actual definition
    pub definition: String,

    /// An example usage
    pub example: String,

    /// The definition permalink
    pub permalink: Url,

    /// ?
    pub sound_urls: Vec<serde_json::Value>,

    /// # of thumbs down
    pub thumbs_down: u64,

    /// # of thumbs up
    pub thumbs_up: u64,

    /// The term
    pub word: String,

    /// Date written
    pub written_on: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Definition {
    /// Get the raw definition.
    pub fn get_raw_definition(&self) -> String {
        self.definition
            .chars()
            .filter(|&c| c != '[' && c != ']')
            .collect()
    }

    /// Get the raw example.
    pub fn get_raw_example(&self) -> String {
        self.example
            .chars()
            .filter(|&c| c != '[' && c != ']')
            .collect()
    }
}
