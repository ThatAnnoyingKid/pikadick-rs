pub mod client;
pub mod types;

pub use crate::client::Client;

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    #[error("{0}")]
    InvalidStatus(reqwest::StatusCode),

    /// Invalid Json
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    /// Invalid Api Error
    #[error("api error ({0})")]
    Api(String),
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
        let data = client.list_random(5).await.unwrap();
        println!("{:#?}", data);
        assert!(!data.is_empty());
    }
}
