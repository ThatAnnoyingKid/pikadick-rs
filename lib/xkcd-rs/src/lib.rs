use reqwest::Url;

/// Library Error type
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Rewqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP StatusCode
    ///
    #[error("{0}")]
    InvalidStatus(reqwest::StatusCode),
}

/// An XKCD client
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

    /// Get a random xkcd comic url.
    ///
    pub async fn get_random(&self) -> Result<Url, Error> {
        let res = self
            .client
            .get("https://c.xkcd.com/random/comic/")
            .send()
            .await?;
        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }
        let ret = res.url().clone();
        let _body = res.text().await?;

        Ok(ret)
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
        let result = client.get_random().await.expect("failed to get xkcd comic");
        dbg!(result);
    }
}
