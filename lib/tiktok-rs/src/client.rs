use crate::{
    types::PostPage,
    Error,
};
use scraper::Html;

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

    /// Get a page as html and parse it
    async fn get_html<F, T>(&self, url: &str, func: F) -> Result<T, Error>
    where
        F: FnOnce(Html) -> T + Send + 'static,
        T: Send + 'static,
    {
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(tokio::task::spawn_blocking(move || {
            let html = Html::parse_document(text.as_str());
            func(html)
        })
        .await?)
    }

    /// Get a tiktock post.
    pub async fn get_post(&self, url: &str) -> Result<PostPage, Error> {
        Ok(self
            .get_html(url, |html| PostPage::from_html(&html))
            .await??)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
