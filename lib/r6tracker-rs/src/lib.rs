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
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Url Parse Error
    #[error(transparent)]
    Url(#[from] url::ParseError),

    /// An API Response returned an error
    #[error("invalid api response")]
    InvalidApiResponse(#[from] InvalidApiResponseError),

    /// An Overwolf Response returned an error.
    #[error("invalid overwolf response")]
    InvalidOverwolfResponse(#[from] InvalidOverwolfResponseError),
}
