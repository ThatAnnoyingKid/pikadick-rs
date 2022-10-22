use crate::OpenGraphObject;
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};

/// An error that may occur while using a [`Client`].
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// A reqwest HTTP error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A tokio task failed
    #[error(transparent)]
    JokioJoin(#[from] tokio::task::JoinError),

    /// The [`OpenGraphObject`] was invalid
    #[error(transparent)]
    InvalidOpenGraphObject(#[from] crate::open_graph_object::FromHtmlError),

    /// An IO Error occured
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An OGP object is missing a video url
    #[error("missing video url")]
    MissingVideoUrl,

    /// An OGP object has an unknown object kind
    #[error("the object kind '{0}' is not supported")]
    UnsupportedObjectKind(String),
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
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let object = tokio::task::spawn_blocking(move || text.parse()).await??;
        Ok(object)
    }

    /// Convenience function for getting data and copying it into an async writer
    pub async fn get_and_copy_to<W>(&self, url: &str, mut writer: W) -> Result<(), ClientError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut response = self.client.get(url).send().await?.error_for_status()?;
        while let Some(chunk) = response.chunk().await? {
            writer.write_all(&chunk).await?;
        }
        Ok(())
    }

    /// Best-effort function to download an [`OpenGraphObject`] and copy it to an async writer.
    ///
    /// If its not good enough, you should probably look at its source and build a custom impl.
    pub async fn download_object_to<W>(
        &self,
        object: &OpenGraphObject,
        writer: W,
    ) -> Result<(), ClientError>
    where
        W: AsyncWrite + Unpin,
    {
        let url = if object.is_video() {
            object
                .video_url
                .as_ref()
                .ok_or(ClientError::MissingVideoUrl)?
                .as_str()
        } else if object.is_image() {
            object.image.as_str()
        } else {
            return Err(ClientError::UnsupportedObjectKind(object.kind.clone()));
        };

        self.get_and_copy_to(url, writer).await?;

        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
