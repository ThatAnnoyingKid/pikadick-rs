use crate::{
    Error,
    types::{
        ApiResponse,
        Platform,
        user_data::UserData,
    },
};
use reqwest::header::{
    ACCEPT,
    HeaderMap,
    HeaderName,
    HeaderValue,
    REFERER,
    USER_AGENT,
};
use serde::de::DeserializeOwned;

static SEC_CH_UA_PLATFORM: HeaderName = HeaderName::from_static("sec-ch-ua-platform");

static REFERER_VALUE: HeaderValue = HeaderValue::from_static("https://r6.tracker.network/");
static USER_AGENT_VALUE: HeaderValue = HeaderValue::from_static(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36",
);
static SEC_CH_UA_PLATFORM_VALUE: HeaderValue = HeaderValue::from_static("\"Windows\"");

/// R6tracker Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(REFERER, REFERER_VALUE.clone());
        default_headers.insert(USER_AGENT, USER_AGENT_VALUE.clone());
        default_headers.insert(SEC_CH_UA_PLATFORM.clone(), SEC_CH_UA_PLATFORM_VALUE.clone());

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()
            .expect("failed to build client");

        Client { client }
    }

    /// Get a url and return it as an [`ApiResponse`].
    async fn get_api_response<T>(&self, url: &str) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        Ok(self
            .client
            .get(url)
            .header(ACCEPT, "application/json, text/plain, */*")
            .send()
            .await?
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

        let platform_str = match platform {
            Platform::Pc => "ubi",
            Platform::Xbox => "xbl",
            Platform::Ps4 => "psn",
        };
        let url = format!(
            "https://api.tracker.gg/api/v2/r6siege/standard/profile/{platform_str}/{name}?"
        );

        self.get_api_response(&url).await
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

    const VALID_USER: &str = "smack.jjfozzil";
    const INVALID_USER: &str = "aaaaabbaaaa";

    // Does not work on CI
    #[tokio::test]
    #[ignore]
    async fn get_profile() {
        let client = Client::new();

        let profile = client
            .get_profile(VALID_USER, Platform::Pc)
            .await
            .expect("failed to get profile")
            .into_result();
        dbg!(profile.unwrap());
    }

    #[tokio::test]
    async fn empty_user() {
        let client = Client::new();

        let profile_err = client.get_profile("", Platform::Pc).await.unwrap_err();
        assert!(matches!(profile_err, Error::EmptyUsername));
    }

    // Does not work on CI
    #[tokio::test]
    #[ignore]
    async fn invalid_user() {
        let client = Client::new();

        let profile_err = client
            .get_profile(INVALID_USER, Platform::Pc)
            .await
            .unwrap()
            .take_invalid()
            .unwrap();
        dbg!(&profile_err);
        assert!(profile_err.is_missing());
    }
}
