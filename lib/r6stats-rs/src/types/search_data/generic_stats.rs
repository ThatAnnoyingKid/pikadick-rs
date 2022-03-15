use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GenericStats {
    pub gamemode: GameMode,
    pub general: General,
    pub queue: Queue,
    pub timestamps: Timestamps,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GameMode {
    pub bomb: Bomb,
    pub hostage: Hostage,
    pub secure_area: SecureArea,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Bomb {
    pub best_score: u32,
    pub games_played: i64,
    pub losses: u32,
    pub playtime: u64,
    pub wins: u32,
    pub wl: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Hostage {
    pub best_score: u32,
    pub extractions_denied: u32,
    pub games_played: i64,
    pub losses: u32,
    pub playtime: u32,
    pub wins: u32,
    pub wl: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SecureArea {
    pub best_score: u32,
    pub games_played: i64,
    pub kills_as_attacker_in_objective: u32,
    pub kills_as_defender_in_objective: u32,
    pub losses: u32,
    pub playtime: u32,
    pub times_objective_secured: u32,
    pub wins: u32,
    pub wl: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct General {
    pub assists: u32,
    pub barricades_deployed: u32,
    pub blind_kills: u32,
    pub bullets_fired: u32,
    pub bullets_hit: u32,
    pub dbnos: u32,
    pub deaths: u32,
    pub distance_travelled: i64,
    pub draws: u32,
    pub gadgets_destroyed: u32,
    pub games_played: u32,
    pub headshots: u32,
    pub kd: f64,
    pub kills: u32,
    pub losses: u32,
    pub melee_kills: u32,
    pub penetration_kills: u32,
    pub playtime: u64,
    pub rappel_breaches: u32,
    pub reinforcements_deployed: u32,
    pub revives: u32,
    pub suicides: u32,
    pub wins: u32,
    pub wl: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Queue {
    pub casual: QueueStat,
    pub other: QueueStat,
    pub ranked: QueueStat,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct QueueStat {
    pub deaths: u32,
    pub draws: u32,
    pub games_played: i64,
    pub kd: f64,
    pub kills: u32,
    pub losses: i64,
    pub playtime: u64,
    pub wins: i64,
    pub wl: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Timestamps {
    #[serde(
        deserialize_with = "time::serde::rfc3339::deserialize",
        serialize_with = "time::serde::rfc3339::serialize"
    )]
    pub created: OffsetDateTime,
    #[serde(
        deserialize_with = "time::serde::rfc3339::deserialize",
        serialize_with = "time::serde::rfc3339::serialize"
    )]
    pub last_updated: OffsetDateTime,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
