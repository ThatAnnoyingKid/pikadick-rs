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
};
use serde::de::DeserializeOwned;
use url::Url;

/// R6tracker Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Get a url and return it as an [`ApiResponse`].
    async fn get_api_response<T>(&self, url: &str) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        Ok(self.client.get(url).send().await?.json().await?)
    }

    /// Get a url and return an [`OverwolfResponse`]
    async fn get_overwolf_response<T>(&self, url: &str) -> Result<OverwolfResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Get an r6tracker profile
    pub async fn get_profile(
        &self,
        name: &str,
        platform: Platform,
    ) -> Result<ApiResponse<UserData>, Error> {
        if name.is_empty() {
            return Err(Error::EmptyUsername);
        }

        let platform = platform.as_u32();
        let url = format!("https://r6.tracker.network/api/v1/standard/profile/{platform}/{name}/");

        self.get_api_response(&url).await
    }

    /// Get the sessions for a user
    pub async fn get_sessions(
        &self,
        name: &str,
        platform: Platform,
    ) -> Result<ApiResponse<SessionsData>, Error> {
        if name.is_empty() {
            return Err(Error::EmptyUsername);
        }

        let platform = platform.as_u32();
        let url = format!(
            "https://r6.tracker.network/api/v1/standard/profile/{platform}/{name}/sessions?"
        );

        self.get_api_response(&url).await
    }

    /// Get player info using the Overwolf API.
    pub async fn get_overwolf_player(
        &self,
        name: &str,
    ) -> Result<OverwolfResponse<OverwolfPlayer>, Error> {
        if name.is_empty() {
            return Err(Error::EmptyUsername);
        }

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

    const VALID_USER: &str = "ALEX-_-SAGAS";
    const INVALID_USER: &str = "aaaaabbaaaa";

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();

        let profile = client
            .get_profile(VALID_USER, Platform::Pc)
            .await
            .expect("failed to get profile")
            .into_result();
        dbg!(profile.unwrap());

        let sessions = client.get_sessions(VALID_USER, Platform::Pc).await.unwrap();
        dbg!(sessions.take_valid().unwrap());
    }

    #[tokio::test]
    async fn it_works_overwolf() {
        let client = Client::new();

        let profile = client
            .get_overwolf_player(VALID_USER)
            .await
            .expect("failed to get overwolf player");
        let profile_data = profile.take_valid().expect("missing profile data");
        dbg!(&profile_data);
    }

    #[tokio::test]
    async fn empty_user() {
        let client = Client::new();

        let profile_err = client.get_profile("", Platform::Pc).await.unwrap_err();
        assert!(matches!(profile_err, Error::EmptyUsername));

        let sessions_err = client.get_sessions("", Platform::Pc).await.unwrap_err();
        assert!(matches!(sessions_err, Error::EmptyUsername));
    }

    #[tokio::test]
    async fn invalid_user() {
        let client = Client::new();

        let profile_err = client
            .get_profile(INVALID_USER, Platform::Pc)
            .await
            .unwrap()
            .take_invalid()
            .unwrap();
        dbg!(profile_err);

        // This errors unpredictably when the user does not exist
        /*
        let sessions_data = client
            .get_sessions(INVALID_USER, Platform::Pc)
            .await
            .unwrap()
            .take_valid()
            .unwrap();
        assert!(sessions_data.items.is_empty());
        dbg!(sessions_data);
        */
    }
}
