/// Overwolf Lifetime Stats
pub mod lifetime_stats;

pub use self::lifetime_stats::LifetimeStats;
use std::collections::HashMap;
use url::Url;

/// The error that occurs when an OverwolfResponse is in an error state and the data field is accessed.
/// It optionally contains the reason of failure.
#[derive(Debug)]
pub struct InvalidOverwolfResponseError(pub Option<String>);

impl std::error::Error for InvalidOverwolfResponseError {}

impl std::fmt::Display for InvalidOverwolfResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(reason) => write!(f, "the overwolf response was invalid ({})", reason),
            None => "the overwolf response was invalid".fmt(f),
        }
    }
}

/// A json Overwolf Response
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OverwolfResponse<T> {
    /// Whether this request was successful. If true, `data` should be Some(T).
    pub success: bool,
    /// The reason of failure if `success` is false.
    pub reason: Option<String>,

    /// The data payload.
    #[serde(flatten)]
    pub data: Option<T>,
}

impl<T> OverwolfResponse<T> {
    /// Get the payload. This checks `success` to ensure `data` is valid and returns an error if it isn't.
    pub fn get_data(&self) -> Result<&T, InvalidOverwolfResponseError> {
        match (self.success, self.reason.as_ref(), self.data.as_ref()) {
            (true, _, Some(data)) => Ok(data),
            (false, reason, _) => Err(InvalidOverwolfResponseError(reason.map(|s| s.to_string()))),
            // This almost certainly will never happen.
            // TODO: Rewrite the deser impl to reject this state as a json error earlier on.
            (true, _, None) => Err(InvalidOverwolfResponseError(None)),
        }
    }

    /// Take the payload, consuming this struct. Same function as `get_data` except it is consuming.
    pub fn take_data(self) -> Result<T, InvalidOverwolfResponseError> {
        match (self.success, self.reason, self.data) {
            (true, _, Some(data)) => Ok(data),
            (false, reason, _) => Err(InvalidOverwolfResponseError(reason)),
            // This almost certainly will never happen.
            // TODO: Rewrite the deser impl to reject this state as a json error earlier on.
            (true, _, None) => Err(InvalidOverwolfResponseError(None)),
        }
    }
}

/// An Overwolf Player
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OverwolfPlayer {
    /// Player ID
    #[serde(rename = "playerId")]
    pub player_id: String,

    /// Player name
    pub name: String,

    /// Avatar URL
    pub avatar: Url,

    /// Player Level
    pub level: u64,

    /// Probably r6tracker premium
    #[serde(rename = "isPremium")]
    pub is_premium: bool,

    /// Whether this person is a suspected cheater
    #[serde(rename = "suspectedCheater")]
    pub suspected_cheater: bool,

    /// The current season
    #[serde(rename = "currentSeason")]
    pub current_season: u64,

    /// Current season best region stats
    #[serde(rename = "currentSeasonBestRegion")]
    pub current_season_best_region: OverwolfSeason,

    /// Lifetime Stats
    #[serde(rename = "lifetimeStats")]
    pub lifetime_stats: LifetimeStats,

    /// All seasonal stats
    pub seasons: Vec<OverwolfSeason>,

    /// Operator Stats
    pub operators: Vec<OverwolfOperator>,

    /// Seasonal Operator Stats
    #[serde(rename = "seasonalOperators")]
    pub seasonal_operators: SeasonalOperators,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Season stats
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OverwolfSeason {
    /// The rank name
    #[serde(rename = "rankName")]
    pub rank_name: String,

    /// Season image URL
    pub img: Url,

    /// Season #
    pub season: u64,

    /// Season Region
    pub region: String,

    /// MMR
    pub mmr: u64,

    /// Win Percent
    #[serde(rename = "winPct")]
    pub win_pct: f64,

    /// The # of wins this season
    pub wins: u64,

    /// The K/D this season
    pub kd: f64,

    /// The # of kills this season
    pub kills: u64,

    /// The # of matches this season
    pub matches: u64,

    /// Maybe the change in mmr this season?
    #[serde(rename = "mmrChange")]
    pub mmr_change: i64,

    /// Current Rank Info
    #[serde(rename = "currentRank")]
    pub current_rank: OverwolfRank,

    /// Previous Rank Info
    #[serde(rename = "prevRank")]
    pub prev_rank: Option<OverwolfRank>,

    /// Next Rank Info
    #[serde(rename = "nextRank")]
    pub next_rank: Option<OverwolfRank>,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Overwolf Rank Info
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OverwolfRank {
    /// Unknown
    #[serde(rename = "rankTier")]
    pub rank_tier: u64,

    /// MMR
    pub mmr: u64,

    /// The icon url for this rank
    #[serde(rename = "rankIcon")]
    pub rank_icon: Url,

    /// The name of this rank
    #[serde(rename = "rankName")]
    pub rank_name: String,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// An Overwolf Operator
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OverwolfOperator {
    /// Operator name
    pub name: String,

    /// Operator Image
    pub img: Url,

    /// Whether this operator is attack
    #[serde(rename = "isAttack")]
    pub is_attack: bool,

    /// Whether this operator is this user's top operator
    #[serde(rename = "isTopOperator")]
    pub is_top_operator: bool,

    /// Win %
    #[serde(rename = "winpct")]
    pub win_pct: f64,

    /// The total # of wins with this op
    pub wins: u64,

    /// The K/D with this op
    pub kd: f64,

    /// The total # of kills with this op
    pub kills: u64,

    /// The time played as a user-displayable string
    #[serde(rename = "timePlayedDisplay")]
    pub time_played_display: String,

    /// The time played (in seconds?)
    #[serde(rename = "timePlayed")]
    pub time_played: u64,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Seasonal Operator data
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SeasonalOperators {
    /// Operator Stats
    pub operators: Vec<OverwolfOperator>,

    /// Started tracking datetimestamp
    #[serde(rename = "startedTracking")]
    pub started_tracking: String,

    /// Unknown keys
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const OVERWOLF_PLAYER: &str = include_str!("../../test_data/overwolf_player.json");
    const INVALID_OVERWOLF_RESPONSE: &str =
        include_str!("../../test_data/invalid_overwolf_response.json");

    #[test]
    fn parse_overwolf_player() {
        let res: OverwolfResponse<OverwolfPlayer> = serde_json::from_str(OVERWOLF_PLAYER).unwrap();
        dbg!(res.data.unwrap());
    }

    #[test]
    fn parse_invalid_overwolf() {
        let res: OverwolfResponse<serde_json::Value> =
            serde_json::from_str(INVALID_OVERWOLF_RESPONSE).unwrap();
        dbg!(res);
    }
}
