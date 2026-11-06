mod client;
pub mod model;

pub use crate::client::Client;

/// Library error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest http error
    #[error("Http error")]
    Reqwest(#[from] reqwest::Error),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn search_users() {
        let client = Client::new();
        let users = client.search_users().await.expect("Failed to search users");
        dbg!(users);
    }
}
