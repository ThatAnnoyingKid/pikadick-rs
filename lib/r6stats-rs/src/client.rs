use crate::{
    types::{
        ApiResponse,
        UserData,
    },
    R6Error,
    R6Result,
};
use bytes::buf::ext::BufExt;
use hyper::StatusCode;
use hyper_tls::HttpsConnector;

#[derive(Debug)]
pub struct Client {
    client: hyper::Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Client { client }
    }

    /// Search pc users for a profile
    pub async fn search(&self, name: &str) -> R6Result<Vec<UserData>> {
        let url = format!("https://r6stats.com/api/player-search/{}/pc", name).parse()?;
        let res = self.client.get(url).await?;

        let status = res.status();
        if !status.is_success() && status != StatusCode::NOT_FOUND {
            return Err(R6Error::InvalidStatus(status));
        }

        let body = hyper::body::aggregate(res.into_body()).await?;

        let res: ApiResponse<Vec<UserData>> = serde_json::from_reader(body.reader())?;

        Ok(res.data)
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
        let client = Client::new();
        let user_list = client.search("KingGeorge").await.unwrap();
        assert!(!user_list.is_empty());
        dbg!(&user_list);
    }

    #[tokio::test]
    async fn invalid_search() {
        let client = Client::new();
        let user_list = client.search("ygwdauiwgd").await.unwrap();
        assert!(user_list.is_empty());
        dbg!(&user_list);
    }
}
