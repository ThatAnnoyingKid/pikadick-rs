mod client;
/// Api Types
pub mod types;

pub use crate::client::Client;

/// Library Result
///
pub type QResult<T> = Result<T, QError>;

/// Library Error
///
#[derive(Debug, thiserror::Error)]
pub enum QError {
    /// Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    ///
    #[error("Invalid HTTP status {0}")]
    InvalidStatus(reqwest::StatusCode),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn check_room() {
        let client = Client::new();
        let data = client.check_room("274218").await.unwrap();

        dbg!(data);
    }
}
