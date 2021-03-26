use crate::{
    Deviation,
    Error,
    OEmbed,
    SearchResults,
};
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

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
            "https://www.deviantart.com/_napi/da-browse/api/networkbar/search/deviations",
            &[("q", query), ("page", "1")],
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
        let url = deviation
            .get_download_url()
            .or_else(|| deviation.get_media_url())
            .ok_or(Error::MissingMediaToken)?;
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
