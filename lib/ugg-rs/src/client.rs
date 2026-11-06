use crate::{
    Error,
    model::{
        GraphQlRequest,
        GraphQlResponse,
        User,
    },
};

const PLAYER_PROFILE_SUGGESTIONS_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/PlayerProfileSuggestions.graphql"
));

/// Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
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
        let response: GraphQlResponse<T> = self
            .client
            .post("https://u.gg/api")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/157.0.0.0 Safari/537.36")
            .json(&GraphQlRequest {
                operation_name,
                query,
                variables,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response.data)
    }

    pub async fn search_users(&self) -> Result<Vec<User>, Error> {
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
                    query: "a",
                    region_id: "na1",
                },
            )
            .await?;

        Ok(response.player_profile_suggestions)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
