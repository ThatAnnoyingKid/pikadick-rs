mod client;
/// Api Types
pub mod types;

pub use self::types::{
    GenericResponse,
    GenericResponseError,
};
pub use crate::client::Client;

/// Library Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid Generic Response
    #[error("invalid generic response")]
    InvalidGenericResponse(#[from] GenericResponseError),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn check_room() {
        let client = Client::new();
        let data = client
            .check_room("274218")
            .await
            .expect("failed to check room");

        dbg!(data);
    }
}
