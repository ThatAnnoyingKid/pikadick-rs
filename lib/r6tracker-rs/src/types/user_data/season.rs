use crate::types::{
    stat::Stat,
    user_data::Rank,
};
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

    /// Get the max mmr
    pub fn max_mmr(&self) -> Option<u64> {
        self.get_stat_by_name("Max MMR").map(|s| s.value as u64)
    }

    /// Get the max rank
    pub fn max_rank(&self) -> Option<Rank> {
        match self.get_stat_by_name("Max Rank")?.value as u64 {
            0 => Some(Rank::Unranked),

            1 => Some(Rank::CopperV),
            2 => Some(Rank::CopperIV),
            3 => Some(Rank::CopperIII),
            4 => Some(Rank::CopperII),
            5 => Some(Rank::CopperI),

            6 => Some(Rank::BronzeV),
            7 => Some(Rank::BronzeIV),
            8 => Some(Rank::BronzeIII),
            9 => Some(Rank::BronzeII),
            10 => Some(Rank::BronzeI),

            11 => Some(Rank::SilverV),
            12 => Some(Rank::SilverIV),
            13 => Some(Rank::SilverIII),
            14 => Some(Rank::SilverII),
            15 => Some(Rank::SilverI),

            16 => Some(Rank::GoldIII),
            17 => Some(Rank::GoldII),
            18 => Some(Rank::GoldI),

            19 => Some(Rank::PlatinumIII),
            20 => Some(Rank::PlatinumII),
            21 => Some(Rank::PlatinumI),

            22 => Some(Rank::Diamond),

            23 => Some(Rank::Champion),

            _ => None,
        }
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
