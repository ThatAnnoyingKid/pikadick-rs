mod client;
mod types;

pub use crate::{
    client::Client,
    types::{
        Image,
        ImageUri,
    },
};
pub use http::uri::InvalidUri;

pub type NekosResult<T> = Result<T, NekosError>;

#[derive(Debug)]
pub enum NekosError {
    InvalidUri(InvalidUri),
    Hyper(hyper::Error),
    Json(serde_json::Error),
    InvalidStatus(hyper::StatusCode),
}

impl std::fmt::Display for NekosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NekosError::InvalidUri(e) => e.fmt(f),
            NekosError::Hyper(e) => e.fmt(f),
            NekosError::Json(e) => e.fmt(f),
            NekosError::InvalidStatus(status) => write!(f, "Invalid Status {}", status),
        }
    }
}

impl std::error::Error for NekosError {}

impl From<http::uri::InvalidUri> for NekosError {
    fn from(e: http::uri::InvalidUri) -> Self {
        NekosError::InvalidUri(e)
    }
}

impl From<hyper::Error> for NekosError {
    fn from(e: hyper::Error) -> NekosError {
        NekosError::Hyper(e)
    }
}

impl From<serde_json::Error> for NekosError {
    fn from(e: serde_json::Error) -> NekosError {
        NekosError::Json(e)
    }
}
