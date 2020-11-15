pub mod client;
pub mod types;

pub use crate::client::Client;

#[derive(Debug)]
pub enum FmlError {
    Http(http::Error),
    Hyper(hyper::Error),
    InvalidHeaderValue(http::header::InvalidHeaderValue),
    InvalidStatus(http::StatusCode),
    Json(serde_json::Error),

    Api(String),
}

impl From<http::Error> for FmlError {
    fn from(e: http::Error) -> Self {
        Self::Http(e)
    }
}

impl From<hyper::Error> for FmlError {
    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}

impl From<http::header::InvalidHeaderValue> for FmlError {
    fn from(e: http::header::InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(e)
    }
}

impl From<serde_json::Error> for FmlError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub type FmlResult<T> = Result<T, FmlError>;
