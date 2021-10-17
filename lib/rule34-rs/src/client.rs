mod query_builder;

pub use self::query_builder::{
    PostListQueryBuilder,
    TagListQueryBuilder,
};
use crate::{
    DeletedImagesList,
    Error,
    HtmlPost,
};
use reqwest::header::{
    HeaderMap,
    HeaderValue,
};
use scraper::Html;
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

/// A Rule34 Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    ///
    /// This probably shouldn't be used by you.
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    pub fn new() -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            HeaderValue::from_static(crate::ACCEPT_LANGUAGE_STR),
        );
        default_headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static(crate::ACCEPT_STR),
        );
        default_headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static(crate::REFERER_STR),
        );

        Client {
            client: reqwest::Client::builder()
                .user_agent(crate::USER_AGENT_STR)
                .default_headers(default_headers)
                .build()
                .expect("failed to build rule34 client"),
        }
    }

    /// Send a GET web request to a `url` and get the result as a [`String`].
    pub async fn get_text(&self, url: &str) -> Result<String, Error> {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    /// Send a GET web request to a `uri` and get the result as [`Html`],
    /// then use the given func to process it.
    pub async fn get_html<F, T>(&self, uri: &str, f: F) -> Result<T, Error>
    where
        F: FnOnce(Html) -> T + Send + 'static,
        T: Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret =
            tokio::task::spawn_blocking(move || f(Html::parse_document(text.as_str()))).await?;
        Ok(ret)
    }

    /// Send a GET web request to a `uri` and get the result as xml, deserializing it to the given type.
    pub async fn get_xml<T>(&self, uri: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret = tokio::task::spawn_blocking(move || quick_xml::de::from_str(&text)).await??;
        Ok(ret)
    }

    /// Create a builder to list posts from rule34.
    pub fn list_posts<'a, 'b>(&'a self) -> PostListQueryBuilder<'a, 'b> {
        PostListQueryBuilder::new(self)
    }

    /// Get a [`HtmlPost`] by `id`.
    pub async fn get_html_post(&self, id: u64) -> Result<HtmlPost, Error> {
        if id == 0 {
            return Err(Error::InvalidId);
        }
        let url = crate::post_id_to_html_post_url(id);
        let ret = self
            .get_html(url.as_str(), |html| HtmlPost::from_html(&html))
            .await??;

        Ok(ret)
    }

    /// Get a list of deleted images.
    ///
    /// Only include ids over `last_id`. Use `None` for no limit.
    /// Due to current technical limitations, this function is not very memory efficient depending on `last_id`.
    /// You should probably limit its use with a semaphore or similar.
    pub async fn get_deleted_images(
        &self,
        last_id: Option<u64>,
    ) -> Result<DeletedImagesList, Error> {
        let mut url = Url::parse_with_params(
            crate::URL_INDEX,
            &[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("deleted", "show"),
            ],
        )?;
        if let Some(last_id) = last_id {
            let mut last_id_buf = itoa::Buffer::new();
            url.query_pairs_mut()
                .append_pair("last_id", last_id_buf.format(last_id));
        }
        // Parse on a threadpool since the full returned string is currently around 30 megabytes in size,
        // and we need to run in under a few milliseconds.
        // We need to buffer this all in memory though, since `quick_xml` does not provide a streaming api.
        self.get_xml(url.as_str()).await
    }

    /// Get a builder to list tags.
    pub fn list_tags(&self) -> TagListQueryBuilder {
        TagListQueryBuilder::new(self)
    }

    /// Send a GET web request to a `uri` and copy the result to the given async writer.
    pub async fn get_to_writer<W>(&self, url: &str, mut writer: W) -> Result<(), Error>
    where
        W: AsyncWrite + Unpin,
    {
        let mut res = self.client.get(url).send().await?.error_for_status()?;

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
            .list_posts()
            .tags(Some("rust"))
            .execute()
            .await
            .expect("failed to search rule34 for `rust`");
        dbg!(&res);
        assert!(!res.posts.is_empty());
    }

    async fn get_top_post(query: &str) -> HtmlPost {
        let client = Client::new();
        let res = client
            .list_posts()
            .tags(Some(query))
            .execute()
            .await
            .unwrap_or_else(|e| panic!("failed to search rule34 for `{}`: {}", query, e));
        assert!(!res.posts.is_empty());

        let first = res.posts.first().expect("missing first entry");
        client
            .get_html_post(first.id)
            .await
            .expect("failed to get first post")
    }

    #[tokio::test]
    async fn it_works() {
        let list = [
            "rust",
            "fbi",
            "gif",
            "corna",
            "sledge",
            "deep",
            "roadhog",
            "deep_space_waifu",
        ];

        for item in list {
            let post = get_top_post(item).await;
            dbg!(&post);
        }
    }

    #[tokio::test]
    async fn deleted_images_list() {
        let client = Client::new();
        let result = client
            .get_deleted_images(Some(500_000)) // Just choose a high-ish post id here and update to keep the download limited
            .await
            .expect("failed to get deleted images");
        dbg!(result);
    }

    #[tokio::test]
    async fn tags_list() {
        let client = Client::new();
        let _result = client
            .list_tags()
            .limit(Some(crate::TAGS_LIST_LIMIT_MAX))
            .execute()
            .await
            .expect("failed to list tags");
        // dbg!(result);
    }
}
