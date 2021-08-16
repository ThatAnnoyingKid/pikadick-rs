/// Api types
pub mod types;

pub use self::types::SearchJson;
use std::sync::Arc;
use url::Url;

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A reqwest error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A URL parse error
    #[error("invalid url")]
    Url(#[from] url::ParseError),
}

/// The sauce nao client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: Arc<str>,
}

impl Client {
    /// Create a new [`Client`].
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Arc::from(api_key),
        }
    }

    /// Look up an image
    pub async fn search(&self, url: &str) -> Result<SearchJson, Error> {
        let url = Url::parse_with_params(
            "https://saucenao.com/search.php?output_type=2",
            &[("api_key", &*self.api_key), ("url", url)],
        )?;
        Ok(self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const API_KEY: &str = include_str!("../api_key.txt");

    #[tokio::test]
    #[ignore]
    async fn search_works() {
        let client = Client::new(API_KEY);
        let results = client
            .search("https://i.imgur.com/oZjCxGo.jpg")
            .await
            .expect("failed to search");
        dbg!(results);
    }
}
