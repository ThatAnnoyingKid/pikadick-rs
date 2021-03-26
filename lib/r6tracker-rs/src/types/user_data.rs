/// Season data type
pub mod season;

pub use self::season::Season;
use crate::{
    types::platform::Platform,
    Stat,
};
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
    /// The error message
    pub message: String,
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

#[allow(clippy::upper_case_acronyms)]
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserData {
    /// Unique user id
    pub id: String,

    #[serde(rename = "type")]
    pub kind: String,

    /// Collection of ranked seasons stats
    pub children: Vec<Season>,

    /// Metadata
    pub metadata: Metadata,

    /// A collection of all stats
    pub stats: Vec<Stat>,

    /// Unknown fields
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl UserData {
    /// Utility function to get a stat by name. Currently an O(n) linear search.
    fn get_stat_by_name(&self, name: &str) -> Option<&Stat> {
        self.stats.iter().find(|s| s.name() == name)
    }

    /// Gets top mmr from all servers.
    pub fn current_mmr(&self) -> Option<u32> {
        self.get_stat_by_name("MMR").map(|s| s.value as u32)
    }

    /// Get the image url for the rank this user is at gloablly
    pub fn current_mmr_image(&self) -> Option<&Url> {
        self.get_stat_by_name("Global MMR")
            .and_then(|s| s.icon_url())
    }

    /// Get the MMR for this user.
    pub fn current_mmr_america(&self) -> Option<u32> {
        self.get_stat_by_name("Global MMR").map(|s| s.value as u32)
    }

    /// Gets this season's color as a string hex value
    pub fn season_color(&self) -> &str {
        &self.metadata.current_season_color
    }

    /// Tries to parse this season's hex color as a u32
    pub fn season_color_u32(&self) -> Option<u32> {
        u32::from_str_radix(self.season_color().get(1..)?, 16).ok()
    }

    /// Get total # of kills
    pub fn get_kills(&self) -> Option<u64> {
        self.get_stat_by_name("Kills").map(|s| s.value as u64)
    }

    /// Get total # of deaths
    pub fn get_deaths(&self) -> Option<u64> {
        self.get_stat_by_name("Deaths").map(|s| s.value as u64)
    }

    /// Get overall K/D
    pub fn kd(&self) -> Option<f64> {
        self.get_stat_by_name("KD Ratio").map(|s| s.value)
    }

    /// Get Overall W/L
    pub fn wl(&self) -> Option<f64> {
        self.get_stat_by_name("WL Ratio").map(|s| s.value)
    }

    /// Get user tag name
    pub fn name(&self) -> &str {
        &self.metadata.platform_user_handle
    }

    /// Get user avatar url
    pub fn avatar_url(&self) -> &Url {
        &self.metadata.picture_url
    }

    /// Get the latest stats for the latest ranked region/season the user has played in
    pub fn get_latest_season(&self) -> Option<&Season> {
        let target_id = format!(
            "region-{}.season-{}",
            self.metadata.latest_region.unwrap_or(100),
            self.metadata.latest_season
        );

        self.children.iter().find(|s| s.id == target_id)
    }

    /// Get the season where the user attained their max ranking
    pub fn get_max_season(&self) -> Option<&Season> {
        self.children
            .iter()
            .filter_map(|child| child.max_mmr().map(|mmr| (child, mmr)))
            .max_by_key(|(_, mmr)| *mmr)
            .map(|(child, _)| child)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    #[serde(rename = "accountId")]
    pub account_id: String,

    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,

    #[serde(rename = "currentSeasonColor")]
    pub current_season_color: String,

    #[serde(rename = "currentSeasonName")]
    pub current_season_name: String,

    #[serde(rename = "latestRegion")]
    pub latest_region: Option<u32>,

    #[serde(rename = "latestSeason")]
    pub latest_season: u32,

    #[serde(rename = "pictureUrl")]
    pub picture_url: Url,

    #[serde(rename = "platformId")]
    pub platform_id: Platform,

    #[serde(rename = "platformUserHandle")]
    pub platform_user_handle: String,

    #[serde(rename = "segmentControls")]
    pub segment_controls: Vec<serde_json::Value>,

    #[serde(rename = "statsCategoryOrder")]
    pub stats_category_order: Vec<String>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::ApiResponse;

    const SAMPLE_1: &str = include_str!("../../test_data/user_data_1.json");
    const SAMPLE_2: &str = include_str!("../../test_data/user_data_2.json");
    const INVALID_USER_DATA: &str = include_str!("../../test_data/invalid_user_data.json");

    #[test]
    fn parse_sample_1() {
        let data = serde_json::from_str::<ApiResponse<UserData>>(SAMPLE_1)
            .unwrap()
            .take_valid()
            .unwrap();
        let season = data.get_latest_season().unwrap();
        dbg!(season);

        let max_season = data.get_max_season().unwrap();
        dbg!(max_season.max_mmr());
        dbg!(max_season.max_rank());
    }

    #[test]
    fn parse_sample_2() {
        let data = serde_json::from_str::<ApiResponse<UserData>>(SAMPLE_2)
            .unwrap()
            .take_valid()
            .unwrap();
        let season = data.get_latest_season().unwrap();

        dbg!(season);
    }

    #[test]
    fn parse_invalid_sample() {
        let data = serde_json::from_str::<ApiResponse<UserData>>(INVALID_USER_DATA).unwrap();

        dbg!(data);
    }
}
