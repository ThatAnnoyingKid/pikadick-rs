mod client;
mod types;

pub use crate::{
    client::Client,
    types::CheckRoomJsonRequest,
};

/// Library Result
pub type QResult<T> = Result<T, QError>;

/// Library Error
#[derive(Debug)]
pub enum QError {
    /// Reqwest HTTP Error
    Reqwest(reqwest::Error),

    /// Invalid HTTP Status
    InvalidStatus(reqwest::StatusCode),
}

impl From<reqwest::Error> for QError {
    fn from(e: reqwest::Error) -> Self {
        QError::Reqwest(e)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn check_room() {
        let client = Client::new();
        let data = client.check_room("114545").await.unwrap();

        dbg!(data);
    }
}
