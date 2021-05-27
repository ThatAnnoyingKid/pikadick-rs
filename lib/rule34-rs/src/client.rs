use crate::{
    types::{
        Post,
        SearchResult,
    },
    RuleError,
};
use bytes::Bytes;
use scraper::Html;
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

const DEFAULT_USER_AGENT_STR: &str = "rule34-rs";

/// A Rule34 Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client. This probably should't be used by you.
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Send a GET web request to a `uri` and get the result as a [`String`].
    pub async fn get_text(&self, uri: &str) -> Result<String, RuleError> {
        Ok(self
            .client
            .get(uri)
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT_STR)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    /// Send a GET web request to a `uri` and get the result as [`Html`],
    /// then use the given func to process it.
    pub async fn get_html<F, T>(&self, uri: &str, f: F) -> Result<T, RuleError>
    where
        F: FnOnce(Html) -> T + Send + 'static,
        T: Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret =
            tokio::task::spawn_blocking(move || f(Html::parse_document(text.as_str()))).await?;
        Ok(ret)
    }

    /// Run a search for a `query`.
    ///
    /// Querys are based on "tags".
    /// Tags are seperated by spaces, while words are seperated by underscores.
    /// Characters are automatically url-encoded.
    ///
    /// Offset is the starting offset in number of results.
    pub async fn search(&self, query: &str, offset: u64) -> Result<SearchResult, RuleError> {
        let mut pid_buffer = itoa::Buffer::new();
        let url = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=list",
            &[("tags", query), ("pid", pid_buffer.format(offset))],
        )?;

        let ret = self
            .get_html(url.as_str(), |html| SearchResult::from_html(&html))
            .await??;

        Ok(ret)
    }

    /// Get a [`Post`] by `id`.
    pub async fn get_post(&self, id: u64) -> Result<Post, RuleError> {
        let mut id_str = itoa::Buffer::new();
        let url = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=view",
            &[("id", id_str.format(id))],
        )?;

        let ret = self
            .get_html(url.as_str(), |html| Post::from_html(&html))
            .await??;

        Ok(ret)
    }

    /// Send a GET web request to a `uri` and download the result as [`Bytes`].
    pub async fn get_bytes(&self, url: &str) -> Result<Bytes, RuleError> {
        Ok(self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT_STR)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?)
    }

    /// Send a GET web request to a `uri` and copy the result to the given async writer.
    pub async fn get_to<W>(&self, url: &Url, mut writer: W) -> Result<(), RuleError>
    where
        W: AsyncWrite + Unpin,
    {
        let mut res = self
            .client
            .get(url.as_str())
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT_STR)
            .send()
            .await?
            .error_for_status()?;

        while let Some(chunk) = res.chunk().await? {
            writer.write_all(&chunk).await?;
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

    #[tokio::test]
    async fn search() {
        let client = Client::new();
        let res = client
            .search("rust", 0)
            .await
            .expect("failed to search rule34 for `rust`");
        dbg!(&res);
        assert!(!res.entries.is_empty());
    }

    async fn get_top_post(query: &str) -> Post {
        let client = Client::new();
        let res = client
            .search(query, 0)
            .await
            .unwrap_or_else(|_| panic!("failed to search rule34 for `{}`", query));
        assert!(!res.entries.is_empty());

        let first = res.entries.first().expect("missing first entry");
        client
            .get_post(first.id)
            .await
            .expect("failed to get first post")
    }

    #[tokio::test]
    async fn it_works_rust() {
        let post = get_top_post("rust").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_fbi() {
        let post = get_top_post("fbi").await;
        assert!(post.thumb_url.is_some());
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_gif() {
        let post = get_top_post("gif").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_corna() {
        let post = get_top_post("corna").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_sledge() {
        let post = get_top_post("sledge").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_deep() {
        let post = get_top_post("deep").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_roadhog() {
        let post = get_top_post("roadhog").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_deep_space_waifu() {
        let post = get_top_post("deep_space_waifu").await;
        dbg!(&post);
    }
}
