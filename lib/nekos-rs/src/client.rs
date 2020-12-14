use crate::{
    types::ImageList,
    NekosError,
    NekosResult,
};
use std::io::Write;
use url::Url;

const DEFAULT_USER_AGENT: &str = "nekos-rs";

/// Client for nekos.moe
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

    /// Get a random list of catgirls.
    /// count is a num from 0 < count <= 100 and is the number of returned images.
    /// nsfw is whether the images should be nsfw. If not specified, both are returned.
    pub async fn get_random(&self, nsfw: Option<bool>, count: u8) -> NekosResult<ImageList> {
        let mut buf = itoa::Buffer::new();
        let count_query = std::iter::once(("count", buf.format(count.min(100))));
        let nsfw_query = nsfw.map(|nsfw| ("nsfw", if nsfw { "true" } else { "false" }));
        let query = count_query.chain(nsfw_query);
        let url = Url::parse_with_params("https://nekos.moe/api/v1/random/image", query)?;

        let res = self
            .client
            .get(url.as_str())
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            return Err(NekosError::InvalidStatus(status));
        }

        let body = res.text().await?;
        let json = serde_json::from_str(&body)?;

        Ok(json)
    }

    /// Get a url and copy it to the given writer
    pub async fn copy_res_to<T: Write>(&self, url: &Url, mut writer: T) -> NekosResult<()> {
        let mut res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(NekosError::InvalidStatus(status));
        }

        while let Some(chunk) = res.chunk().await? {
            writer.write_all(&chunk)?;
        }

        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
