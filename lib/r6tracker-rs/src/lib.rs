pub mod client;
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

use http::uri::InvalidUri;
pub use hyper::StatusCode;

pub type R6Result<T> = Result<T, R6Error>;

#[derive(Debug)]
pub enum R6Error {
    InvalidStatus(StatusCode),

    InvalidUri(InvalidUri),
    Hyper(hyper::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for R6Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            R6Error::InvalidStatus(status) => write!(f, "Invalid Status: {}", status),
            R6Error::InvalidUri(e) => e.fmt(f),
            R6Error::Hyper(e) => e.fmt(f),
            R6Error::Json(e) => e.fmt(f),
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
        Self::InvalidUri(e)
    }
}

impl From<serde_json::Error> for R6Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
