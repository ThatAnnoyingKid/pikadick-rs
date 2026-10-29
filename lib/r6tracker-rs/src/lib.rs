mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::{
        InvalidApiResponseError,
        Platform,
        SegmentSeason,
        UserData,
    },
};

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

    /// The username is empty
    #[error("the username is empty")]
    EmptyUsername,
}
