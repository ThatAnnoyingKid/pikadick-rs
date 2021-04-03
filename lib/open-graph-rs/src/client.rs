use crate::OpenGraphObject;

/// An error that may occur while using a [`Client`].
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// A reqwest HTTP error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    #[error("invalid HTTP status '{0}'")]
    InvalidStatus(reqwest::StatusCode),

    /// A tokio task failed
    #[error(transparent)]
    JokioJoin(#[from] tokio::task::JoinError),

    /// The [`OpenGraphObject`] was invalid
    #[error(transparent)]
    InvalidOpenGraphObject(#[from] crate::open_graph_object::FromHtmlError),
}

/// A generic open graph protocol client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner HTTP Client.
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    pub fn new() -> Self {
        Self::from_client(Default::default())
    }

    /// Make a new [`Client`] from a [`reqwest::Client`].
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Get an [`OpenGraphObject`] by url.
    pub async fn get_object(&self, url: &str) -> Result<OpenGraphObject, ClientError> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ClientError::InvalidStatus(status));
        }
        let text = response.text().await?;
        let object = tokio::task::spawn_blocking(move || text.parse()).await??;
        Ok(object)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
