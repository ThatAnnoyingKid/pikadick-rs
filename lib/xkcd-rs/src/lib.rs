use reqwest::Url;

/// Library Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Rewqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

/// An XKCD client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Get a random xkcd comic url.
    pub async fn get_random(&self) -> Result<Url, Error> {
        // We can't use head, we get rejected with 405.
        let res = self
            .client
            .get("https://c.xkcd.com/random/comic/")
            .send()
            .await?
            .error_for_status()?;
        // Stash the final url we get redirected to.
        let ret = res.url().clone();

        // We do this to be nice,
        // and not just cancel the body as some servers may freak out.
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
        dbg!(&result);
    }
}
