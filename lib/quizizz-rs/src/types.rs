use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CheckRoomJsonRequest<'a> {
    #[serde(rename = "roomCode")]
    pub room_code: &'a str,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CheckRoomObfuscatedJsonResponse {
    pub odata: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GenericResponse {
    #[serde(rename = "__cid__")]
    pub cid: serde_json::Value,
    pub error: Option<serde_json::Value>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
