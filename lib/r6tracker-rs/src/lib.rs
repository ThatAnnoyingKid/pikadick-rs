mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::{
        InvalidApiResponseError,
        InvalidOverwolfResponseError,
        OverwolfPlayer,
        OverwolfResponse,
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

    /// An API Response returned an error
    #[error("{0}")]
    InvalidApiResponse(#[from] InvalidApiResponseError),

    /// An Overwolf Response returned an error.
    #[error("{0}")]
    InvalidOverwolfResponse(#[from] InvalidOverwolfResponseError),

    /// Too short of a name was provided. The member is the length of the erroneous name
    #[error("the name is too short")]
    InvalidNameLength(usize),
}
