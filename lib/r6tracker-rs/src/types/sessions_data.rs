use crate::types::stat::Stat;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SessionsData {
    pub items: Vec<Session>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Session {
    #[serde(rename = "startedAt")]
    pub started_at: String,

    #[serde(rename = "endedAt")]
    pub ended_at: Option<String>,

    pub duration: f64,

    #[serde(rename = "isActive")]
    pub is_active: bool,

    pub matches: Vec<Match>,
    pub stats: Vec<Stat>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Match {
    pub id: String,

    #[serde(rename = "type")]
    pub kind: String,

    pub metadata: serde_json::Value,
    pub stats: Vec<Stat>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::ApiResponse;

    const SAMPLE_1: &str = include_str!("../../test_data/sessions_data.json");

    #[test]
    fn parse_sample_1() {
        let data = serde_json::from_str::<ApiResponse<SessionsData>>(SAMPLE_1)
            .unwrap()
            .take_valid()
            .unwrap();

        dbg!(&data);
    }
}
