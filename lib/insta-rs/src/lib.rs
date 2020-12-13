/// Api Types
pub mod types;

pub use crate::types::Post;
use select::document::Document;

/// Result type
pub type InstaResult<T> = Result<T, InstaError>;

/// Error
#[derive(Debug, thiserror::Error)]
pub enum InstaError {
    /// Reqwest Http Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP status
    #[error("invalid status {0}")]
    InvalidStatus(reqwest::StatusCode),

    /// Missing a HTML element. Internal string is mostly for debugging.
    #[error("missing html element")]
    MissingElement(&'static str),
}

/// Client
#[derive(Debug)]
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

    /// Get a post
    pub async fn get_post(&self, url: &str) -> InstaResult<Post> {
        let res = self.client.get(url).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(InstaError::InvalidStatus(status));
        }
        let text = res.text().await?;
        let doc = Document::from(text.as_str());
        let post = Post::from_doc(&doc)?;

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
