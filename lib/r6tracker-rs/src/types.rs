/// Overwolf API Types
pub mod overwolf;
pub mod platform;
pub mod sessions_data;
pub mod stat;
pub mod user_data;

pub use self::{
    overwolf::{
        OverwolfOperator,
        OverwolfPlayer,
        OverwolfRank,
        OverwolfResponse,
        OverwolfSeason,
    },
    platform::Platform,
    sessions_data::SessionsData,
    stat::Stat,
    user_data::UserData,
};
use std::collections::HashMap;

/// A json response from the API.
#[derive(serde::Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub data: T,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
