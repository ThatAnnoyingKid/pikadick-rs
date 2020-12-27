pub mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::UserData,
};

pub type R6Result<T> = Result<T, R6Error>;

/// Library Error Type
#[derive(Debug)]
pub enum R6Error {
    /// Reqwest HTTP Error
    Reqwest(reqwest::Error),

    /// Json Error
    Json(serde_json::Error),

    UnknownJson(serde_json::Value),
}

impl std::fmt::Display for R6Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            R6Error::Reqwest(e) => e.fmt(f),
            R6Error::Json(e) => e.fmt(f),
            R6Error::UnknownJson(json) => write!(f, "Unknown Json Response: {}", json),
        }
    }
}

impl std::error::Error for R6Error {}

impl From<reqwest::Error> for R6Error {
    fn from(e: reqwest::Error) -> Self {
        R6Error::Reqwest(e)
    }
}

impl From<serde_json::Error> for R6Error {
    fn from(e: serde_json::Error) -> Self {
        R6Error::Json(e)
    }
}
