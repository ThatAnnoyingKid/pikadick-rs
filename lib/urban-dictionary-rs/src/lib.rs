mod client;
mod types;

pub use crate::{
    client::Client,
    types::{
        Definition,
        DefinitionList,
    },
};

/// Library Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A reqwest http error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A url parse error
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let result = client.lookup("smol").await.expect("invalid response");
        dbg!(result);
    }
}
