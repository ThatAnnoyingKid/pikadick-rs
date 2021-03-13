/// Library Types
///
mod types;

pub use crate::types::{
    OpenGraphObject,
    PostUrl,
};
use select::document::Document;
use tokio::io::AsyncWriteExt;

/// Error Type
///
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status Code
    ///
    #[error("invalid status code '{0}'")]
    InvalidStatus(reqwest::StatusCode),

    /// A Tokio Task Panicked
    ///
    #[error("{0}")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Failed to parse an [`OpenGraphObject`].
    ///
    #[error("{0}")]
    InvalidOpenGraphObject(#[from] crate::types::open_graph_object::FromDocError),

    /// Io Error
    ///
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// A tiktok client
///
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner HTTP client.
    ///
    /// Should only be used if you want to piggyback off of this for HTTP requests
    ///
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    ///
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.86 Safari/537.36")
                .cookie_store(true)
                .build()
                .expect("failed to build client"),
        }
    }

    /// Get a tiktock post.
    ///
    pub async fn get_post(&self, url: &PostUrl) -> Result<OpenGraphObject, Error> {
        let res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        let text = res.text().await?;

        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }

        let ret = tokio::task::spawn_blocking(move || {
            let doc = Document::from(text.as_str());
            OpenGraphObject::from_doc(&doc)
        })
        .await??;

        Ok(ret)
    }

    /// Send a HTTP request to the url and copy the response to the given writer
    ///
    pub async fn get_to<W>(&self, url: &str, mut writer: W) -> Result<(), Error>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut res = self
            .client
            .get(url)
            .header("referer", "https://www.tiktok.com/")
            .send()
            .await?;
        let status = res.status();
        while let Some(bytes) = res.chunk().await? {
            writer.write_all(&bytes).await?;
        }
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }

        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use url::Url;

    #[tokio::test]
    async fn download() {
        let url = Url::parse("https://www.tiktok.com/@silksheets/video/6916308321234341125")
            .expect("invalid url");
        let url = PostUrl::from_url(url).expect("invalid media url");
        let client = Client::new();

        let post = client.get_post(&url).await.expect("failed to get post");

        dbg!(&post);
        dbg!(&post.video_url);
    }
}
