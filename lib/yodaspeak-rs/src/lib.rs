mod client;

pub use self::client::Client;

/// The error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// The translate result is missing
    #[error("missing result")]
    MissingResult,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let input = "this translator works";
        let translated = client.translate(input).await.expect("failed to translate");
        dbg!(translated);
    }
}
