use std::collections::HashMap;
use url::Url;

/// A json response from the UserData API.
#[derive(Debug)]
pub enum ApiResponse<T> {
    /// A Valid Response
    Valid(T),

    /// An Invalid Response
    Invalid(InvalidApiResponseError),
}

#[derive(Debug)]
pub struct InvalidApiResponseError(pub Vec<ApiError>);

impl InvalidApiResponseError {
    /// Returns true if this is a missing error.
    pub fn is_missing(&self) -> bool {
        match self.0.as_slice() {
            [first] => first.is_missing(),
            _ => false,
        }
    }
}

impl std::error::Error for InvalidApiResponseError {}

impl std::fmt::Display for InvalidApiResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "the api request failed due to the following: ")?;
        for error in self.0.iter() {
            writeln!(f, "    {}", error.message)?;
        }

        Ok(())
    }
}

/// Errors that occured while procesing an API Request
#[derive(serde::Deserialize, Debug)]
pub struct ApiError {
    /// The error code string.
    pub code: String,

    /// The error message
    pub message: String,
}

impl ApiError {
    /// Returns true if this is a missing error.
    pub fn is_missing(&self) -> bool {
        // The user does not exist.
        self.code == "CollectorResultStatus::NotFound" || 
        // The user exists but has not played siege.
        self.code == "CollectorResultStatus::NoData"
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "api error ({})", self.message)
    }
}

impl std::error::Error for ApiError {}

impl<'de, T> serde::Deserialize<'de> for ApiResponse<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut map = serde_json::Map::deserialize(deserializer)?;

        let data: Option<Result<T, _>> = map
            .remove("data")
            .map(|data| serde::Deserialize::deserialize(data).map_err(serde::de::Error::custom));
        let rest = serde_json::Value::Object(map);

        match data {
            Some(data) => Ok(Self::Valid(data?)),
            None => {
                #[derive(serde::Deserialize)]
                struct ErrorReason {
                    errors: Vec<ApiError>,
                }

                ErrorReason::deserialize(rest)
                    .map(|e| Self::Invalid(InvalidApiResponseError(e.errors)))
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

impl<T> ApiResponse<T> {
    /// Convert this into as Result.
    pub fn into_result(self) -> Result<T, InvalidApiResponseError> {
        match self {
            Self::Valid(data) => Ok(data),
            Self::Invalid(err) => Err(err),
        }
    }

    /// Consume self and return the valid variant, or None.
    pub fn take_valid(self) -> Option<T> {
        match self {
            Self::Valid(data) => Some(data),
            Self::Invalid(_) => None,
        }
    }

    /// Consume self and return the invalid variant, or None.
    pub fn take_invalid(self) -> Option<InvalidApiResponseError> {
        match self {
            Self::Valid(_) => None,
            Self::Invalid(err) => Some(err),
        }
    }
}

/// An R6 Rank.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Rank {
    Unranked,

    CopperV,
    CopperIV,
    CopperIII,
    CopperII,
    CopperI,

    BronzeV,
    BronzeIV,
    BronzeIII,
    BronzeII,
    BronzeI,

    SilverV,
    SilverIV,
    SilverIII,
    SilverII,
    SilverI,

    GoldIII,
    GoldII,
    GoldI,

    PlatinumIII,
    PlatinumII,
    PlatinumI,

    Diamond,

    Champion,
}

impl Rank {
    /// Get a string rep of this rank
    pub fn name(self) -> &'static str {
        match self {
            Self::Unranked => "Unranked",

            Self::CopperV => "Copper V",
            Self::CopperIV => "Copper IV",
            Self::CopperIII => "Copper III",
            Self::CopperII => "Copper II",
            Self::CopperI => "Copper I",

            Self::BronzeV => "Bronze V",
            Self::BronzeIV => "Bronze IV",
            Self::BronzeIII => "Bronze III",
            Self::BronzeII => "Bronze II",
            Self::BronzeI => "Bronze I",

            Self::SilverV => "Silver V",
            Self::SilverIV => "Silver IV",
            Self::SilverIII => "Silver III",
            Self::SilverII => "Silver II",
            Self::SilverI => "Silver I",

            Self::GoldIII => "Gold III",
            Self::GoldII => "Gold II",
            Self::GoldI => "Gold I",

            Self::PlatinumIII => "Platinum III",
            Self::PlatinumII => "Platinum II",
            Self::PlatinumI => "Platinum I",

            Self::Diamond => "Diamond",

            Self::Champion => "Champion",
        }
    }
}

fn parse_hex_color(color: &str) -> Option<u32> {
    u32::from_str_radix(color.strip_prefix('#')?, 16).ok()
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NumberPrecision2Stat {
    pub value: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NumberPercentageStat {
    pub value: f64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentOverviewStats {
    #[serde(rename = "kdRatio")]
    pub kd_ratio: NumberPrecision2Stat,
    #[serde(rename = "winPercentage")]
    pub win_percentage: NumberPercentageStat,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentOverview {
    pub stats: SegmentOverviewStats,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl SegmentOverview {
    /// Get the k/d ratio.
    pub fn kd_ratio_value(&self) -> f64 {
        self.stats.kd_ratio.value
    }

    /// Get the win percentage.
    pub fn win_percentage_value(&self) -> f64 {
        self.stats.win_percentage.value
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentGameModeAttributes {
    /// The game mode
    #[serde(rename = "gamemode")]
    pub game_mode: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentGameModeStats {
    #[serde(rename = "kdRatio")]
    pub kd_ratio: NumberPrecision2Stat,
    #[serde(rename = "winPercentage")]
    pub win_percentage: NumberPercentageStat,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentGameMode {
    pub attributes: SegmentGameModeAttributes,
    pub stats: SegmentGameModeStats,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl SegmentGameMode {
    /// Returns `true` if this is Ranked.
    pub fn is_ranked(&self) -> bool {
        self.attributes.game_mode == "pvp_ranked"
    }

    /// Get the k/d ratio.
    pub fn kd_ratio_value(&self) -> f64 {
        self.stats.kd_ratio.value
    }

    /// Get the win percentage.
    pub fn win_percentage_value(&self) -> f64 {
        self.stats.win_percentage.value
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    #[serde(rename = "currentSeason")]
    pub current_season: u16,

    #[serde(rename = "clearanceLevel")]
    pub clearance_level: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentSeasonAttributes {
    /// The season number
    pub season: u16,

    /// The game mode
    #[serde(rename = "gamemode")]
    pub game_mode: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentSeasonMetadata {
    /// The hex color of this season.
    pub color: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct MmrStatMetadata {
    #[serde(rename = "imageUrl")]
    pub image_url: Url,
    pub name: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct MmrStat {
    pub value: Option<u32>,
    pub metadata: MmrStatMetadata,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RankPointsStatMetadata {
    #[serde(rename = "imageUrl")]
    pub image_url: Url,
    pub name: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RankPointsStat {
    pub value: Option<u32>,
    pub metadata: RankPointsStatMetadata,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NumberStat {
    pub value: u64,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentSeasonStats {
    pub mmr: Option<MmrStat>,
    #[serde(rename = "maxMmr")]
    pub max_mmr: Option<MmrStat>,
    #[serde(rename = "rankPoints")]
    pub rank_points: Option<RankPointsStat>,
    #[serde(rename = "maxRankPoints")]
    pub max_rank_points: Option<RankPointsStat>,
    #[serde(rename = "kdRatio")]
    pub kd_ratio: NumberPrecision2Stat,
    #[serde(rename = "winPercentage")]
    pub win_percentage: NumberPercentageStat,
    #[serde(rename = "matchesPlayed")]
    pub matches_played: NumberStat,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SegmentSeason {
    pub attributes: SegmentSeasonAttributes,
    pub metadata: SegmentSeasonMetadata,
    pub stats: SegmentSeasonStats,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl SegmentSeason {
    /// Returns `true` if this is Ranked.
    pub fn is_ranked(&self) -> bool {
        self.attributes.game_mode == "pvp_ranked"
    }

    /// Returns `true` if this is Casual.
    pub fn is_casual(&self) -> bool {
        self.attributes.game_mode == "pvp_casual"
    }

    /// Returns `true` if this is Unranked.
    pub fn is_unranked(&self) -> bool {
        self.attributes.game_mode == "pvp_standard"
    }

    /// Tries to parse this season's hex color as a u32
    pub fn color_u32(&self) -> Option<u32> {
        parse_hex_color(self.metadata.color.as_str())
    }

    /// Get the max mmr value.
    pub fn max_mmr_value(&self) -> Option<u32> {
        self.stats.max_mmr.as_ref()?.value
    }

    /// Get the max rp value.
    pub fn max_rank_points_value(&self) -> Option<u32> {
        self.stats.max_rank_points.as_ref()?.value
    }

    /// Get the max ranking value, mmr or rp.
    pub fn max_ranking_value(&self) -> Option<u32> {
        self.max_mmr_value().or(self.max_rank_points_value())
    }

    /// Get the max mmr rank name.
    pub fn max_mmr_rank_name(&self) -> Option<&str> {
        self.stats.max_mmr.as_ref()?.metadata.name.as_deref()
    }

    /// Get the max rp rank name.
    pub fn max_rank_points_rank_name(&self) -> Option<&str> {
        self.stats
            .max_rank_points
            .as_ref()?
            .metadata
            .name
            .as_deref()
    }

    /// Get the max ranking name, mmr or rp.
    pub fn max_ranking_rank_name(&self) -> Option<&str> {
        self.max_mmr_rank_name()
            .or(self.max_rank_points_rank_name())
    }

    /// Get current mmr value.
    pub fn mmr_value(&self) -> Option<u32> {
        self.stats.mmr.as_ref()?.value
    }

    /// Get current ranking points value.
    pub fn ranking_points_value(&self) -> Option<u32> {
        self.stats.rank_points.as_ref()?.value
    }

    /// Get current ranking value, mmr or rp.
    pub fn ranking_value(&self) -> Option<u32> {
        self.mmr_value().or(self.ranking_points_value())
    }

    /// Get current mmr rank name.
    pub fn mmr_rank_name(&self) -> Option<&str> {
        self.stats.mmr.as_ref()?.metadata.name.as_deref()
    }

    /// Get current ranking rank name.
    pub fn ranking_points_rank_name(&self) -> Option<&str> {
        self.stats.rank_points.as_ref()?.metadata.name.as_deref()
    }

    /// Get current ranking name, mmr or rp.
    pub fn ranking_rank_name(&self) -> Option<&str> {
        self.mmr_rank_name().or(self.ranking_points_rank_name())
    }

    /// Get the k/d ratio.
    pub fn kd_ratio_value(&self) -> f64 {
        self.stats.kd_ratio.value
    }

    /// Get the win percentage.
    pub fn win_percentage_value(&self) -> f64 {
        self.stats.win_percentage.value
    }

    /// Get the # of matches played.
    pub fn matches_played_value(&self) -> u64 {
        self.stats.matches_played.value
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Segment {
    #[serde(rename = "overview")]
    Overview(SegmentOverview),

    #[serde(rename = "gamemode")]
    GameMode(Box<SegmentGameMode>),

    #[serde(rename = "season")]
    Season(Box<SegmentSeason>),
}

impl Segment {
    /// Get a ref to the SegmentOverview if it is an overview.
    pub fn overview_ref(&self) -> Option<&SegmentOverview> {
        match self {
            Self::Overview(value) => Some(value),
            _ => None,
        }
    }

    /// Get a ref to the SegmentGameMode if it is a game mode.
    pub fn game_mode_ref(&self) -> Option<&SegmentGameMode> {
        match self {
            Self::GameMode(value) => Some(value),
            _ => None,
        }
    }

    /// Get a ref to the SegmentSeason if it is a season.
    pub fn season_ref(&self) -> Option<&SegmentSeason> {
        match self {
            Self::Season(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserDataPlatformInfo {
    #[serde(rename = "platformUserHandle")]
    pub platform_user_handle: String,

    #[serde(rename = "avatarUrl")]
    pub avatar_url: String,

    /// Unknown fields
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserDataUserInfo {
    #[serde(rename = "isSuspicious")]
    pub is_suspicious: Option<bool>,

    /// Unknown fields
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserData {
    /// Metadata
    pub metadata: Metadata,

    pub segments: Vec<Segment>,
    #[serde(rename = "platformInfo")]
    pub platform_info: UserDataPlatformInfo,

    #[serde(rename = "userInfo")]
    pub user_info: UserDataUserInfo,

    /// Unknown fields
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl UserData {
    /// Get the current ranked season.
    pub fn get_current_ranked_season(&self) -> Option<&SegmentSeason> {
        self.segments
            .iter()
            .filter_map(|segment| segment.season_ref())
            .filter(|season| season.is_ranked())
            .find(|season| season.attributes.season == self.metadata.current_season)
    }

    /// Get the current unranked season.
    pub fn get_current_unranked_season(&self) -> Option<&SegmentSeason> {
        self.segments
            .iter()
            .filter_map(|segment| segment.season_ref())
            .filter(|season| season.is_unranked())
            .find(|season| season.attributes.season == self.metadata.current_season)
    }

    /// Get the current casual season.
    pub fn get_current_casual_season(&self) -> Option<&SegmentSeason> {
        self.segments
            .iter()
            .filter_map(|segment| segment.season_ref())
            .filter(|season| season.is_casual())
            .find(|season| season.attributes.season == self.metadata.current_season)
    }

    /// Get the ranked season where the user attained their max ranking, either mmr or rp.
    pub fn get_max_ranked_season(&self) -> Option<&SegmentSeason> {
        self.segments
            .iter()
            .filter_map(|segment| segment.season_ref())
            .filter(|season| season.is_ranked())
            .filter(|season| {
                season
                    .max_mmr_value()
                    .or(season.max_rank_points_value())
                    .is_some()
            })
            .max_by_key(|season| {
                season
                    .max_mmr_value()
                    .or(season.max_rank_points_value())
                    .expect("season has no ranking")
            })
    }

    /// Get the image url for the rank this user.
    pub fn current_rank_points_image(&self) -> Option<&Url> {
        Some(
            &self
                .get_current_ranked_season()?
                .stats
                .rank_points
                .as_ref()?
                .metadata
                .image_url,
        )
    }

    /// Get the lifetime ranked stats.
    pub fn get_ranked_game_mode(&self) -> Option<&SegmentGameMode> {
        self.segments
            .iter()
            .filter_map(|segment| segment.game_mode_ref())
            .find(|game_mode| game_mode.is_ranked())
    }

    /// Get overview stats.
    pub fn get_overview(&self) -> Option<&SegmentOverview> {
        self.segments
            .iter()
            .find_map(|segment| segment.overview_ref())
    }
}
