mod client;
pub mod model;

pub use crate::client::Client;

/// Library error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest http error
    #[error("Http error")]
    Reqwest(#[from] reqwest::Error),

    /// Json error
    #[error("Json error")]
    Json(#[from] serde_json::Error),

    /// GraphQl error
    #[error("GraphQl error")]
    GraphQl(#[from] self::model::graphql::GraphQlResponseError),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn search_users() {
        let client = Client::new();
        let users = client
            .search_users("a")
            .await
            .expect("Failed to search users");
        dbg!(users);
    }

    #[tokio::test]
    async fn fetch_match_summaries() {
        let client = Client::new();
        let users = client
            .fetch_match_summaries()
            .await
            .expect("Failed to fetch match summaries");
        dbg!(users);
    }
}
