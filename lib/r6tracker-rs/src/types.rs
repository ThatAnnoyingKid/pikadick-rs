pub mod platform;
pub mod sessions_data;
pub mod stat;
pub mod user_data;

pub use self::{
    platform::Platform,
    sessions_data::SessionsData,
    stat::Stat,
    user_data::UserData,
};
use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
