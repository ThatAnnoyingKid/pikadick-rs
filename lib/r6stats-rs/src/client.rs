use crate::{
    types::{
        ApiResponse,
        UserData,
    },
    Error,
};
use std::time::Duration;

/// An R6Stats client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build client");

        Client { client }
    }

    // TODO: Add non-pc support
    /// Search for a PC user's profile by name.
    pub async fn search(&self, name: &str) -> Result<Vec<UserData>, Error> {
        let url = format!("https://r6stats.com/api/player-search/{name}/pc");
        let text = self.client.get(&url).send().await?.text().await?;
        let response: ApiResponse<Vec<UserData>> = serde_json::from_str(&text)?;

        Ok(response.data)
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

    // 9/1/2023: Online is currently broken, ignore
    #[tokio::test]
    #[ignore]
    async fn it_works() {
        let client = Client::new();
        let user_list = client.search("KingGeorge").await.unwrap();
        assert!(!user_list.is_empty());
        dbg!(&user_list);
    }

    // 9/1/2023: Online is currently broken, ignore
    #[tokio::test]
    #[ignore]
    async fn invalid_search() {
        let client = Client::new();
        let user_list = client.search("ygwdauiwgd").await.unwrap();
        assert!(user_list.is_empty());
        dbg!(&user_list);
    }
}
