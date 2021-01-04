pub mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::UserData,
};

/// Library Result Type
pub type R6Result<T> = Result<T, Error>;

/// Library Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Json Error
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}
