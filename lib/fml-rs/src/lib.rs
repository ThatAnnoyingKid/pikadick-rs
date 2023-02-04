#![allow(clippy::uninlined_format_args)]

/// Client type
pub mod client;
/// API Types
pub mod types;

pub use crate::client::Client;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid Json
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Invalid Api Error
    #[error("api error ({0})")]
    Api(String),

    /// An API response was invalid
    #[error("invalid api response")]
    InvalidApiResponse,
}

/// Result Type
pub type FmlResult<T> = Result<T, Error>;

#[cfg(test)]
mod test {
    use super::*;

    const KEY: &str = include_str!("../key.txt");

    #[tokio::test]
    async fn random() {
        let client = Client::new(KEY.into());
        let data = client.list_random(5).await.expect("invalid list");
        println!("{:#?}", data);
        assert!(!data.is_empty());
    }
}
