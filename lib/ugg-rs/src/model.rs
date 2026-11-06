#[derive(Debug, serde::Serialize)]
pub(crate) struct GraphQlRequest<'a, V> {
    #[serde(rename = "operationName")]
    pub(crate) operation_name: &'a str,
    pub(crate) query: &'a str,
    pub(crate) variables: V,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct GraphQlResponse<T> {
    pub(crate) data: T,
}

#[derive(Debug, serde::Deserialize)]
pub struct User {
    pub puuid: String,

    #[serde(rename = "riotTagLine")]
    pub riot_tag_line: String,

    #[serde(rename = "riotUserName")]
    pub riot_user_name: String,

    #[serde(rename = "summonerLevel")]
    pub summoner_level: u32,

    pub tier: String,

    pub lp: u32,

    #[serde(rename = "iconId")]
    pub icon_id: u32,
}
