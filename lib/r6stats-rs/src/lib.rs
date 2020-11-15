pub mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::UserData,
};
pub use http::uri::InvalidUri;
pub use hyper::StatusCode;

pub type R6Result<T> = Result<T, R6Error>;

/// Library Error Type
#[derive(Debug)]
pub enum R6Error {
    Hyper(hyper::Error),
    InvalidUri(InvalidUri),
    InvalidStatus(StatusCode),
    Json(serde_json::error::Error),

    UnknownJson(serde_json::Value),
}

impl std::fmt::Display for R6Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            R6Error::Hyper(e) => e.fmt(f),
            R6Error::InvalidUri(e) => e.fmt(f),
            R6Error::InvalidStatus(status) => write!(f, "Invalid Status {}", status),
            R6Error::Json(e) => e.fmt(f),
            R6Error::UnknownJson(json) => write!(f, "Unknown Json Response: {}", json),
        }
    }
}

impl std::error::Error for R6Error {}

impl From<hyper::Error> for R6Error {
    fn from(e: hyper::Error) -> Self {
        R6Error::Hyper(e)
    }
}

impl From<InvalidUri> for R6Error {
    fn from(e: InvalidUri) -> Self {
        R6Error::InvalidUri(e)
    }
}

impl From<serde_json::Error> for R6Error {
    fn from(e: serde_json::Error) -> Self {
        R6Error::Json(e)
    }
}
