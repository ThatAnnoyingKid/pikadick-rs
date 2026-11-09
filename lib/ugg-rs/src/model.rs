pub mod graphql;

pub(crate) use self::graphql::GraphQlResponse;

#[derive(Debug, serde::Serialize)]
pub(crate) struct GraphQlRequest<'a, V> {
    #[serde(rename = "operationName")]
    pub(crate) operation_name: &'a str,
    pub(crate) query: &'a str,
    pub(crate) variables: V,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct User {
    pub puuid: String,

    #[serde(rename = "riotTagLine")]
    pub riot_tag_line: String,

    #[serde(rename = "riotUserName")]
    pub riot_user_name: String,

    #[serde(rename = "summonerLevel")]
    pub summoner_level: u32,

    pub tier: Option<String>,

    pub rank: Option<String>,

    pub lp: Option<u32>,

    #[serde(rename = "iconId")]
    pub icon_id: u32,

    #[serde(rename = "regionId")]
    pub region_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct StatusWithErrorReason {
    #[serde(rename = "errorReason")]
    pub error_reason: Option<serde_json::Value>,

    pub success: bool,
}
