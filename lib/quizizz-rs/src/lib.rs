pub mod client;
pub mod obfs;
pub mod types;

pub use crate::{
    client::Client,
    types::CheckRoomJsonRequest,
};

pub type QResult<T> = Result<T, QError>;

#[derive(Debug)]
pub enum QError {
    Http(http::Error),
    InvalidStatus(http::StatusCode),
    Hyper(hyper::Error),
    Json(serde_json::Error),

    Decode,
}

impl From<http::Error> for QError {
    fn from(e: http::Error) -> Self {
        QError::Http(e)
    }
}

impl From<hyper::Error> for QError {
    fn from(e: hyper::Error) -> Self {
        QError::Hyper(e)
    }
}

impl From<serde_json::Error> for QError {
    fn from(e: serde_json::Error) -> Self {
        QError::Json(e)
    }
}
