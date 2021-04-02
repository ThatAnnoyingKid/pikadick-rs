pub use open_graph::{
    self,
    OpenGraphObject,
};

/// Result type
pub type InstaResult<T> = Result<T, InstaError>;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum InstaError {
    /// Reqwest Http Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP status
    #[error("invalid status {0}")]
    InvalidStatus(reqwest::StatusCode),

    /// A Tokio Task Panicked
    #[error("{0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// Failed to parse an [`OpenGraphObject`].
    #[error("{0}")]
    InvalidOpenGraphObject(#[from] open_graph::open_graph_object::FromHtmlError),
}

/// A Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    ///
    /// Useful only for piggybacking requests off of. It's generally a bad idea to touch it.
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Get a post by url.
    pub async fn get_post(&self, url: &str) -> InstaResult<OpenGraphObject> {
        let res = self.client.get(url).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(InstaError::InvalidStatus(status));
        }
        let text = res.text().await?;
        let post = tokio::task::spawn_blocking(move || {
            let doc = open_graph::Html::parse_document(text.as_str());
            InstaResult::Ok(OpenGraphObject::from_html(&doc)?)
        })
        .await??;

        Ok(post)
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
            .unwrap();
        dbg!(res);
    }
}
