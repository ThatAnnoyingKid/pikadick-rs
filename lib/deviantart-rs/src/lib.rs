/// Client
///
pub mod client;
/// Api Types
///
pub mod types;

pub use crate::{
    client::Client,
    types::{
        Deviation,
        OEmbed,
        SearchResults,
    },
};

/// Library Error
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status Code
    ///
    #[error("{0}")]
    InvalidStatus(reqwest::StatusCode),

    /// Invalid Url
    ///
    #[error("{0}")]
    Url(#[from] url::ParseError),

    /// A tokio task panicked
    ///
    #[error("0")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Missing media token
    ///
    #[error("missing media token")]
    MissingMediaToken,

    /// Io Error
    ///
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let results = client.search("sun").await.expect("failed to search");
        // dbg!(&results);
        let first = &results.deviations[0];
        dbg!(first);
        let image = tokio::fs::File::create("test.jpg").await.unwrap();
        client.download_deviation(first, image).await.unwrap();
    }
}
