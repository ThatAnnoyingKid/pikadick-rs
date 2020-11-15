pub mod generic_stats;

pub use self::generic_stats::GenericStats;
use chrono::{
    DateTime,
    Utc,
};
use std::collections::HashMap;
use url::Url;

/// Api Response
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// User Data
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserData {
    pub avatar_banned: bool,
    pub avatar_url_146: Url,
    pub avatar_url_256: Url,
    pub claimed: bool,

    #[serde(rename = "genericStats")]
    pub generic_stats: Option<GenericStats>,

    pub last_updated: DateTime<Utc>,
    pub platform: String,

    #[serde(rename = "progressionStats")]
    pub progression_stats: Option<ProgressionStats>,

    #[serde(rename = "seasonalStats")]
    pub seasonal_stats: Option<SeasonalStats>,

    pub ubisoft_id: String,
    pub uplay_id: String,

    pub username: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl UserData {
    pub fn kd(&self) -> Option<f64> {
        Some(self.generic_stats.as_ref()?.general.kd)
    }

    pub fn wl(&self) -> Option<f64> {
        Some(self.generic_stats.as_ref()?.general.wl)
    }

    pub fn mmr(&self) -> Option<u32> {
        Some(self.seasonal_stats.as_ref()?.mmr as u32)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProgressionStats {
    pub level: u32,
    pub lootbox_probability: u32,
    pub total_xp: u64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SeasonalStats {
    pub abandons: u32,
    pub champions_rank_position: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub created_for_date: DateTime<Utc>,
    pub deaths: Option<u32>,
    pub kills: Option<u32>,
    pub last_match_mmr_change: Option<i32>,
    pub last_match_skill_mean_change: Option<f64>,
    pub last_match_skill_standard_deviation_change: Option<f64>,
    pub losses: u32,
    pub max_mmr: f64,
    pub max_rank: u32,
    pub mmr: f64,
    pub next_rank_mmr: u32,
    pub prev_rank_mmr: u32,
    pub rank: u32,
    pub region: String,
    pub skill_mean: f64,
    pub skill_standard_deviation: f64,
    pub updated_at: DateTime<Utc>,
    pub wins: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const VALID_DATA: &str = include_str!("../../test_data/search_data_valid.json");
    const SEARCH_ASDF_DATA: &str = include_str!("../../test_data/search_asdf.json");

    #[tokio::test]
    async fn parse_valid() {
        let valid = serde_json::from_str::<Vec<UserData>>(VALID_DATA).unwrap();
        dbg!(&valid);
    }

    #[tokio::test]
    async fn parse_asdf() {
        let valid = serde_json::from_str::<Vec<UserData>>(SEARCH_ASDF_DATA).unwrap();
        dbg!(&valid);
    }
}
