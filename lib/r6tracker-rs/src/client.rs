use crate::{
    types::{
        sessions_data::SessionsData,
        user_data::UserData,
        ApiResponse,
        OverwolfPlayer,
        OverwolfResponse,
        Platform,
    },
    Error,
    R6Result,
};
use serde::de::DeserializeOwned;
use url::Url;

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

    /// Get a url and return it as an ApiResponse.
    async fn get_api_response<T: DeserializeOwned>(&self, url: &str) -> R6Result<ApiResponse<T>> {
        let res = self.client.get(url).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        let text = res.text().await?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Get a url and return an Overwolf API Response
    async fn get_overwolf_response<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> R6Result<OverwolfResponse<T>> {
        let res = self.client.get(url).send().await?;
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

    /// Get player info using the Overwolf API.
    pub async fn get_overwolf_player(
        &self,
        name: &str,
    ) -> R6Result<OverwolfResponse<OverwolfPlayer>> {
        let url = Url::parse_with_params(
            "https://r6.tracker.network/api/v0/overwolf/player",
            &[("name", name)],
        )?;

        self.get_overwolf_response(url.as_str()).await
    }

    // TODO: Investigate https://r6.tracker.network/api/v0/overwolf/operators
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const VALID_USER: &str = "KingGeorge";
    const INVALID_USER: &str = "";

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();

        let profile = client.get_profile(VALID_USER, Platform::Pc).await.unwrap();
        assert!(profile.unknown.is_empty());
        dbg!(profile.data);

        let sessions = client.get_sessions(VALID_USER, Platform::Pc).await.unwrap();
        assert!(sessions.unknown.is_empty());
        dbg!(sessions.data);
    }

    #[tokio::test]
    async fn it_works_overwolf() {
        let client = Client::new();

        let profile = client.get_overwolf_player(VALID_USER).await.unwrap();
        let profile_data = profile.take_data().unwrap();
        dbg!(&profile_data);
    }

    #[tokio::test]
    async fn empty_user() {
        let client = Client::new();

        let profile_err = client
            .get_profile(INVALID_USER, Platform::Pc)
            .await
            .unwrap_err();
        assert!(matches!(
            profile_err,
            Error::InvalidStatus(reqwest::StatusCode::NOT_FOUND)
        ));
        dbg!(profile_err);

        let sessions_err = client
            .get_sessions(INVALID_USER, Platform::Pc)
            .await
            .unwrap_err();
        assert!(matches!(
            sessions_err,
            Error::InvalidStatus(reqwest::StatusCode::NOT_FOUND)
        ));
        dbg!(sessions_err);
    }
}
