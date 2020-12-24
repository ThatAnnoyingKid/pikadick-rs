use crate::{
    types::{
        sessions_data::SessionsData,
        user_data::UserData,
        ApiResponse,
        Platform,
    },
    Error,
    R6Result,
};
use serde::de::DeserializeOwned;

/// R6tracker Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Get a url and return it as a json object
    async fn get_api_response<T: DeserializeOwned>(&self, uri: &str) -> R6Result<ApiResponse<T>> {
        let res = self.client.get(uri).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        let text = res.text().await?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Get an r6tracker profile
    pub async fn get_profile(
        &self,
        name: &str,
        platform: Platform,
    ) -> R6Result<ApiResponse<UserData>> {
        let uri = format!(
            "https://r6.tracker.network/api/v1/standard/profile/{}/{}",
            platform.as_u32(),
            name
        );
        self.get_api_response(&uri).await
    }

    /// Get the sessions for a user
    pub async fn get_sessions(
        &self,
        name: &str,
        platform: Platform,
    ) -> R6Result<ApiResponse<SessionsData>> {
        let uri = format!(
            "https://r6.tracker.network/api/v1/standard/profile/{}/{}/sessions?",
            platform.as_u32(),
            name
        );
        self.get_api_response(&uri).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let user = "KingGeorge";
        let client = Client::new();

        let profile = client.get_profile(user, Platform::Pc).await.unwrap();
        assert!(profile.unknown.is_empty());
        dbg!(profile.data);

        let sessions = client.get_sessions(user, Platform::Pc).await.unwrap();
        assert!(sessions.unknown.is_empty());
        dbg!(sessions.data);
    }

    #[tokio::test]
    async fn empty_user() {
        let user = "";
        let client = Client::new();

        let profile_err = client.get_profile(user, Platform::Pc).await.unwrap_err();
        assert!(matches!(
            profile_err,
            Error::InvalidStatus(reqwest::StatusCode::NOT_FOUND)
        ));
        dbg!(profile_err);

        let sessions_err = client.get_sessions(user, Platform::Pc).await.unwrap_err();
        assert!(matches!(
            sessions_err,
            Error::InvalidStatus(reqwest::StatusCode::NOT_FOUND)
        ));
        dbg!(sessions_err);
    }
}
