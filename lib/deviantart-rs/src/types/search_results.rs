use super::Deviation;
use std::collections::HashMap;

/// DeviantArt Search Results
#[derive(Debug, serde::Deserialize)]
pub struct SearchResults {
    /// Deviations
    pub deviations: Vec<Deviation>,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
