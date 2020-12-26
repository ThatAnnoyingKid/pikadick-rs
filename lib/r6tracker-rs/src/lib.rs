mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::{
        Platform,
        SessionsData,
        Stat,
        UserData,
    },
};
pub use reqwest::StatusCode;

/// Result type
pub type R6Result<T> = Result<T, Error>;

/// Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    #[error("invalid http status {0}")]
    InvalidStatus(reqwest::StatusCode),

    /// Json Error
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    /// Url Parse Error
    #[error("{0}")]
    Url(#[from] url::ParseError),
}
