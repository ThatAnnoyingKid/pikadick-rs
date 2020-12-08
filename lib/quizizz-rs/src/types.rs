use std::collections::HashMap;

/// Check Room Request
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CheckRoomJsonRequest<'a> {
    /// Room code
    #[serde(rename = "roomCode")]
    pub room_code: &'a str,
}

/// Api Response
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GenericResponse {
    /// Unknown
    #[serde(rename = "__cid__")]
    pub cid: serde_json::Value,

    /// Error
    pub error: Option<serde_json::Value>,

    /// Other
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
