use crate::{
    types::{
        sessions_data::SessionsData,
        user_data::UserData,
        ApiResponse,
        Platform,
    },
    R6Error,
    R6Result,
};
use bytes::buf::ext::BufExt;
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct Client {
    client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Client { client }
    }

    async fn get_api_response<T: DeserializeOwned>(&self, uri: &str) -> R6Result<ApiResponse<T>> {
        let uri = uri.parse()?;
        let res = self.client.get(uri).await?;

        let status = res.status();
        if !status.is_success() {
            return Err(R6Error::InvalidStatus(status));
        }

        let body = hyper::body::aggregate(res.into_body()).await?;

        Ok(serde_json::from_reader(body.reader())?)
    }

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
}
