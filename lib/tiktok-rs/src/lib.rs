pub use open_graph::{
    self,
    Html,
    OpenGraphObject,
};

/// Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A Tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Failed to parse an [`OpenGraphObject`].
    #[error("invalid ogp object")]
    InvalidOpenGraphObject(#[from] open_graph::open_graph_object::FromHtmlError),
}

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.86 Safari/537.36";

/// A tiktok client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner HTTP client.
    ///
    /// Should only be used if you want to piggyback off of this for HTTP requests
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(USER_AGENT_STR)
                .cookie_store(true)
                .use_rustls_tls() // native-tls chokes for some reason
                .build()
                .expect("failed to build client"),
        }
    }

    /// Get a tiktock post.
    pub async fn get_post(&self, url: &str) -> Result<OpenGraphObject, Error> {
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let ret = tokio::task::spawn_blocking(move || {
            let doc = Html::parse_document(text.as_str());
            OpenGraphObject::from_html(&doc)
        })
        .await??;

        Ok(ret)
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

    // Only works locally
    #[tokio::test]
    #[ignore]
    async fn download() {
        let url = "https://vm.tiktok.com/TTPdrksrdc/";
        let client = Client::new();

        let post = client.get_post(&url).await.expect("failed to get post");

        dbg!(&post);
        dbg!(&post.video_url);
    }
}
