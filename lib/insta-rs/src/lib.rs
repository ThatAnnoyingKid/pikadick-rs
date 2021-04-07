pub use open_graph::{
    self,
    OpenGraphObject,
};

/// Result type
pub type InstaResult<T> = Result<T, InstaError>;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum InstaError {
    /// Failed to fetch and decode an [`OpenGraphObject`].
    #[error(transparent)]
    OpenGraphClient(#[from] open_graph::client::ClientError),
}

/// A Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner open graph client.
    pub client: open_graph::Client,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Client {
            client: open_graph::Client::new(),
        }
    }

    /// Get a post by url.
    pub async fn get_post(&self, url: &str) -> InstaResult<OpenGraphObject> {
        Ok(self.client.get_object(url).await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Fails on CI since other people hit the rate limit.
    #[ignore]
    #[tokio::test]
    async fn get_post() {
        let client = Client::new();
        let res = client
            .get_post("https://www.instagram.com/p/CIlZpXKFfNt/")
            .await
            .expect("failed to get post");
        dbg!(res);
    }
}
