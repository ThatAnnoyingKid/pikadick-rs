use crate::{
    Error,
    model::{
        GraphQlRequest,
        GraphQlResponse,
        StatusWithErrorReason,
        User,
    },
};

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/157.0.0.0 Safari/537.36";

const PLAYER_PROFILE_SUGGESTIONS_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/PlayerProfileSuggestions.graphql"
));

const UPDATE_PLAYER_PROFILE_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/UpdatePlayerProfile.graphql"
));

const FETCH_MATCH_SUMMARIES_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/FetchMatchSummaries.graphql"
));

/// Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    pub client: reqwest::Client,
}

impl Client {
    /// Create a new client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn make_graphql_request<T, V>(
        &self,
        operation_name: &str,
        query: &str,
        variables: V,
    ) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
        V: serde::Serialize,
    {
        let response = self
            .client
            .post("https://u.gg/api")
            .header(reqwest::header::USER_AGENT, USER_AGENT_STR)
            .json(&GraphQlRequest {
                operation_name,
                query,
                variables,
            })
            .send()
            .await?
            .error_for_status()?;
        let text = response.text().await?;
        // dbg!(&text);
        let response: GraphQlResponse<T> = serde_json::from_str(&text)?;

        Ok(response.into_result()?)
    }

    /// Search for users.
    pub async fn search_users(&self, query: &str) -> Result<Vec<User>, Error> {
        #[derive(Debug, serde::Serialize)]
        struct PlayerProfileSuggestionsVariables<'a> {
            query: &'a str,
            #[serde(rename = "regionId")]
            region_id: &'a str,
        }

        #[derive(Debug, serde::Deserialize)]
        struct PlayerProfileSuggestionsResponse {
            #[serde(rename = "playerProfileSuggestions")]
            player_profile_suggestions: Vec<User>,
        }

        let response: PlayerProfileSuggestionsResponse = self
            .make_graphql_request(
                "PlayerProfileSuggestions",
                PLAYER_PROFILE_SUGGESTIONS_QUERY,
                PlayerProfileSuggestionsVariables {
                    query,
                    region_id: "na1",
                },
            )
            .await?;

        Ok(response.player_profile_suggestions)
    }

    pub async fn update_player_profile(
        &self,
        user_name: &str,
        tag_line: &str,
    ) -> Result<(), Error> {
        #[derive(Debug, serde::Serialize)]
        struct UpdatePlayerProfileVariables<'a> {
            #[serde(rename = "riotUserName")]
            riot_user_name: &'a str,

            #[serde(rename = "riotTagLine")]
            riot_tag_line: &'a str,

            #[serde(rename = "regionId")]
            region_id: &'a str,
        }

        #[derive(Debug, serde::Deserialize)]
        struct UpdatePlayerProfileResponse {
            #[serde(rename = "updatePlayerProfile")]
            update_player_profile: StatusWithErrorReason,
        }

        let response: UpdatePlayerProfileResponse = self
            .make_graphql_request(
                "UpdatePlayerProfile",
                UPDATE_PLAYER_PROFILE_QUERY,
                UpdatePlayerProfileVariables {
                    riot_user_name: user_name,
                    riot_tag_line: tag_line,
                    region_id: "na1",
                },
            )
            .await?;
        if !response.update_player_profile.success {
            return Err(Error::Generic("API call failed"));
        }

        Ok(())
    }

    pub async fn fetch_match_summaries(&self) -> Result<(), Error> {
        #[derive(Debug, serde::Serialize)]
        struct FetchMatchSummariesVariables<'a> {
            #[serde(rename = "championId")]
            champion_id: Vec<()>,

            #[serde(rename = "duoRiotTagLine")]
            duo_riot_tag_line: &'static str,

            #[serde(rename = "duoRiotUserName")]
            duo_riot_user_name: &'static str,

            page: u32,

            #[serde(rename = "queueType")]
            queue_type: Vec<()>,

            #[serde(rename = "regionId")]
            region_id: &'static str,

            #[serde(rename = "riotTagLine")]
            riot_tag_line: &'a str,

            #[serde(rename = "riotUserName")]
            riot_user_name: &'a str,

            role: Vec<()>,

            #[serde(rename = "seasonIds")]
            season_ids: Vec<u64>,
        }

        #[derive(Debug, serde::Deserialize)]
        struct FetchMatchSummariesResponse {
            #[serde(rename = "fetchPlayerMatchSummaries")]
            fetch_match_summaries: serde_json::Value,
        }

        let response: FetchMatchSummariesResponse = self
            .make_graphql_request(
                "FetchMatchSummaries",
                FETCH_MATCH_SUMMARIES_QUERY,
                FetchMatchSummariesVariables {
                    champion_id: Vec::new(),
                    duo_riot_tag_line: "",
                    duo_riot_user_name: "",
                    page: 1,
                    queue_type: Vec::new(),
                    region_id: "na1",
                    riot_tag_line: "1234",
                    riot_user_name: "adumbidiot",
                    role: Vec::new(),
                    season_ids: vec![26, 25],
                },
            )
            .await?;

        dbg!(response.fetch_match_summaries);
        todo!("fetch_match_summaries")
        // Ok(response.fetch_match_summaries)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
