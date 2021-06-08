use crate::DefinitionList;
use url::Url;

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

/// Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Lookup a term
    pub async fn lookup(&self, term: &str) -> Result<DefinitionList, Error> {
        let url = Url::parse_with_params(
            "https://api.urbandictionary.com/v0/define",
            &[("term", term)],
        )?;
        Ok(self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
