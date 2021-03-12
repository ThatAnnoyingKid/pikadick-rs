use std::collections::HashMap;
use url::Url;

/// Library Error Type
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// URL Parse Error
    ///
    #[error("{0}")]
    Url(#[from] url::ParseError),

    /// Invalid HTTP Status
    ///
    #[error("invalid status code '{0}'")]
    InvalidStatus(reqwest::StatusCode),
}

/// Client
///
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    ///
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Lookup a term.
    ///
    pub async fn lookup(&self, term: &str) -> Result<DefinitionList, Error> {
        let url = Url::parse_with_params(
            "https://api.urbandictionary.com/v0/define",
            &[("term", term)],
        )?;
        let res = self.client.get(url.as_str()).send().await?;
        let status = res.status();

        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }

        Ok(res.json().await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// A List of [`Definition`].
///
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DefinitionList {
    /// The inner list
    ///
    pub list: Vec<Definition>,

    /// Unknown k/vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A [`Definition`] for a term.
///
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Definition {
    /// The author
    ///
    pub author: String,

    /// The current votes for this
    ///
    pub current_vote: String,

    /// The definition id
    ///
    pub defid: u64,

    /// The actual definition
    ///
    pub definition: String,

    /// An example usage
    ///
    pub example: String,

    /// The definition permalink
    ///
    pub permalink: Url,

    /// ?
    ///
    pub sound_urls: Vec<serde_json::Value>,

    /// # of thumbs down
    ///
    pub thumbs_down: u64,

    /// # of thumbs up
    ///
    pub thumbs_up: u64,

    /// The term
    ///
    pub word: String,

    /// Date written
    ///
    pub written_on: String,

    /// Unknown K/Vs
    ///
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Definition {
    /// Get the raw definition.
    ///
    pub fn get_raw_definition(&self) -> String {
        self.definition
            .chars()
            .filter(|&c| c != '[' && c != ']')
            .collect()
    }

    /// Get the raw example.
    ///
    pub fn get_raw_example(&self) -> String {
        self.example
            .chars()
            .filter(|&c| c != '[' && c != ']')
            .collect()
    }
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
