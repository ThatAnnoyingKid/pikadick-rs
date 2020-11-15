use crate::types::stat::Stat;
use std::collections::HashMap;

/// A representation of a ranked season/region
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Season {
    pub id: String,

    #[serde(rename = "type")]
    pub kind: String,

    pub metadata: Metadata,
    pub stats: Vec<Stat>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Season {
    /// Utility function to get a stat by name. Currently an O(n) linear search.
    fn get_stat_by_name(&self, name: &str) -> Option<&Stat> {
        self.stats.iter().find(|s| s.name() == name)
    }

    /// Gets current mmr for this region in this season
    pub fn current_mmr(&self) -> Option<u32> {
        self.get_stat_by_name("MMR").map(|s| s.value as u32)
    }

    /// Get Win / Loss this season/region
    pub fn wl(&self) -> Option<f64> {
        // Why is this different from UserData?
        self.get_stat_by_name("WLRatio").map(|s| s.value)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    pub name: String,
    pub segment: String,

    #[serde(rename = "statsCategoryOrder")]
    pub stats_category_order: Vec<String>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
