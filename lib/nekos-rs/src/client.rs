use crate::{
    types::ImageList,
    Error,
};
use std::time::Duration;
use url::Url;

const DEFAULT_USER_AGENT: &str = "nekos-rs";

/// Client for nekos.moe
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(10))
                .user_agent(DEFAULT_USER_AGENT)
                .build()
                .expect("failed to build client"),
        }
    }

    /// Get a random list of catgirls.
    ///
    /// count is a num from 0 < count <= 100 and is the number of returned images.
    /// nsfw is whether the images should be nsfw. If not specified, both are returned.
    pub async fn get_random(&self, nsfw: Option<bool>, count: u8) -> Result<ImageList, Error> {
        let mut buf = itoa::Buffer::new();
        let count_query = std::iter::once(("count", buf.format(count.min(100))));
        let nsfw_query = nsfw.map(|nsfw| ("nsfw", if nsfw { "true" } else { "false" }));
        let query = count_query.chain(nsfw_query);
        let url = Url::parse_with_params("https://nekos.moe/api/v1/random/image", query)?;

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

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
