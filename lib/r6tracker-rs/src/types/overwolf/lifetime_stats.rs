use std::collections::HashMap;
use url::Url;

/// Player Lifetime Stats
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LifetimeStats {
    /// Best MMR Stats
    #[serde(rename = "bestMmr")]
    pub best_mmr: Option<BestMmr>,

    /// Win Percent
    #[serde(rename = "winPct")]
    pub win_pct: f64,

    /// Total # of wins
    pub wins: u64,

    /// Total K/D
    pub kd: f64,

    /// Total # of kills
    pub kills: u64,

    /// Total # of matches
    pub matches: u64,

    /// Total headshot %
    #[serde(rename = "headshotPct")]
    pub headshot_pct: f64,

    /// Total # of headshots
    pub headshots: u64,

    /// Total # of melee kills
    #[serde(rename = "meleeKills")]
    pub melee_kills: u64,

    /// Total # of blind kills
    #[serde(rename = "blindKills")]
    pub blind_kills: u64,

    /// Total # of deaths
    pub deaths: u64,

    /// Total # of losses
    pub losses: u64,

    /// Total # of XP
    pub xp: u64,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Best Overwolf Lifetime MMR
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BestMmr {
    /// MMR
    pub mmr: u64,

    /// Rank Name
    pub name: String,

    /// Rank Image URL
    pub img: Url,

    /// Unknown Keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
