/// Api Types
///
pub mod types;

pub use crate::types::{
    Deviation,
    OEmbed,
    SearchResults,
};
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

/// Library Error
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status Code
    ///
    #[error("{0}")]
    InvalidStatus(reqwest::StatusCode),

    /// Invalid Url
    ///
    #[error("{0}")]
    Url(#[from] url::ParseError),

    /// A tokio task panicked
    ///
    #[error("0")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Missing media token
    ///
    #[error("missing media token")]
    MissingMediaToken,

    /// Io Error
    ///
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// A DeviantArt Client
///
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    ///
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Search for deviations
    ///
    pub async fn search(&self, query: &str) -> Result<SearchResults, Error> {
        let url = Url::parse_with_params(
            "https://www.deviantart.com/_napi/da-browse/api/faceted",
            &[
                ("init", "false"),
                ("page_type", "deviations"),
                ("order", "popular-all-time"),
                ("include_scraps", "true"),
                ("q", query),
                ("offset", "0"),
            ],
        )?;
        let res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        let results: SearchResults = res.json().await?;

        Ok(results)
    }

    /// OEmbed API
    ///
    pub async fn get_oembed(&self, url: &Url) -> Result<OEmbed, Error> {
        let url = Url::parse_with_params(
            "https://backend.deviantart.com/oembed",
            &[("url", url.as_str())],
        )?;
        let res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        Ok(res.json().await?)
    }

    /// Download a [`Deviation`].
    ///
    pub async fn download_deviation(
        &self,
        deviation: &Deviation,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<(), Error> {
        let url = deviation.get_media_url().ok_or(Error::MissingMediaToken)?;
        let mut res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        while let Some(chunk) = res.chunk().await? {
            writer.write(&chunk).await?;
        }
        writer.flush().await?;

        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let results = client.search("sun").await.unwrap();
        // dbg!(&results);
        let first = &results.deviations[0];
        dbg!(first);
        let image = tokio::fs::File::create("test.jpg").await.unwrap();
        client.download_deviation(first, image).await.unwrap();
    }
}
