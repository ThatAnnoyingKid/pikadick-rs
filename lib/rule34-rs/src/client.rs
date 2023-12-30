mod note_list_query_builder;
mod post_list_query_builder;
mod tag_list_query_builder;

pub use self::{
    note_list_query_builder::NotesListQueryBuilder,
    post_list_query_builder::PostListQueryBuilder,
    tag_list_query_builder::TagListQueryBuilder,
};
#[cfg(feature = "scrape")]
use crate::HtmlPost;
use crate::{
    DeletedImageList,
    Error,
};
use reqwest::header::{
    HeaderMap,
    HeaderValue,
};
#[cfg(feature = "scrape")]
use scraper::Html;
use std::{
    num::NonZeroU64,
    time::Duration,
};
use url::Url;

// Default Header values
static USER_AGENT_VALUE: HeaderValue = HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4514.0 Safari/537.36");
static REFERER_VALUE: HeaderValue = HeaderValue::from_static("https://rule34.xxx/");
static ACCEPT_LANGUAGE_VALUE: HeaderValue = HeaderValue::from_static("en,en-US;q=0,5");
static ACCEPT_VALUE: HeaderValue = HeaderValue::from_static("*/*");

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
        default_headers.insert(reqwest::header::USER_AGENT, USER_AGENT_VALUE.clone());
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            ACCEPT_LANGUAGE_VALUE.clone(),
        );
        default_headers.insert(reqwest::header::ACCEPT, ACCEPT_VALUE.clone());
        default_headers.insert(reqwest::header::REFERER, REFERER_VALUE.clone());

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build rule34 client");

        Client { client }
    }

    /// Send a GET web request to a `url` and get the result as a [`String`].
    async fn get_text(&self, url: &str) -> Result<String, Error> {
        Ok(self
            .client
            .get(url)
            .timeout(Duration::from_secs(90))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    /// Send a GET web request to a `uri` and get the result as [`Html`],
    /// then use the given func to process it.
    #[cfg(feature = "scrape")]
    async fn get_html<F, T>(&self, uri: &str, f: F) -> Result<T, Error>
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
    async fn get_xml<T>(&self, uri: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret = tokio::task::spawn_blocking(move || quick_xml::de::from_str(&text)).await??;
        Ok(ret)
    }

    /// Create a builder to list posts from rule34.
    pub fn list_posts(&self) -> PostListQueryBuilder {
        PostListQueryBuilder::new(self)
    }

    /// Get a [`HtmlPost`] by `id`.
    #[cfg(feature = "scrape")]
    pub async fn get_html_post(&self, id: NonZeroU64) -> Result<HtmlPost, Error> {
        let url = crate::post_id_to_html_post_url(id);
        let ret = self
            .get_html(url.as_str(), |html| HtmlPost::from_html(&html))
            .await??;

        Ok(ret)
    }

    /// Get a list of deleted images.
    ///
    /// Only include ids over `last_id`. Use `None` for no limit.
    ///
    /// # Warning
    /// Due to current technical limitations,
    /// this function is not very memory efficient depending on `last_id`.
    /// This will require buffering ~30MB into memory.
    /// You should probably limit its use with a semaphore or similar.
    pub async fn list_deleted_images(
        &self,
        last_id: Option<NonZeroU64>,
    ) -> Result<DeletedImageList, Error> {
        let mut url = Url::parse_with_params(
            crate::API_BASE_URL,
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
                .append_pair("last_id", last_id_buf.format(last_id.get()));
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

    /// Get a builder to list notes.
    ///
    /// This is undocumented.
    pub fn list_notes(&self) -> NotesListQueryBuilder {
        NotesListQueryBuilder::new(self)
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

    async fn get_top_post(query: &str) {
        let client = Client::new();
        let response = client
            .list_posts()
            .tags(Some(query))
            .limit(Some(crate::POST_LIST_LIMIT_MAX))
            .execute()
            .await
            .unwrap_or_else(|error| panic!("failed to search rule34 for \"{query}\": {error}"));
        assert!(!response.posts.is_empty(), "no posts for \"{query}\"");

        dbg!(&response);

        #[cfg(feature = "scrape")]
        {
            let first = response.posts.first().expect("missing first entry");
            let post = client
                .get_html_post(first.id)
                .await
                .expect("failed to get first post");
            dbg!(post);
        }
    }

    #[tokio::test]
    async fn it_works() {
        let list = [
            "rust",
            "fbi",
            "gif",
            "corna",
            "sledge",
            "roadhog",
            "deep_space_waifu",
            "aokuro",
        ];

        for item in list {
            get_top_post(item).await;
        }
    }

    #[tokio::test]
    async fn deleted_images_list() {
        let client = Client::new();
        let result = client
            .list_deleted_images(Some(NonZeroU64::new(826_550).unwrap())) // Just choose a high-ish post id here and update to keep the download limited
            .await
            .expect("failed to get deleted images");
        dbg!(result);
    }

    #[tokio::test]
    async fn tags_list() {
        let client = Client::new();
        let result = client
            .list_tags()
            .limit(Some(crate::TAGS_LIST_LIMIT_MAX))
            .order(Some("name"))
            .execute()
            .await
            .expect("failed to list tags");
        assert!(!result.tags.is_empty());
        // dbg!(result);
    }

    #[tokio::test]
    async fn bad_tags_list() {
        let tags = [
            "swallow_(pokémon_move)",
            "akoúo̱_(rwby)",
            "miló_(rwby)",
            "las_tres_niñas_(company)",
        ];

        let client = Client::new();
        for expected_tag_name in tags {
            let tags = client
                .list_tags()
                .name(Some(expected_tag_name))
                .execute()
                .await
                .expect("failed to get tag")
                .tags;
            let tags_len = tags.len();

            assert!(
                tags_len == 1,
                "tags does not have one tag, it has {tags_len} tags"
            );
            let tag = tags.first().expect("tag list is empty");
            let actual_tag_name = &*tag.name;

            assert!(
                actual_tag_name == expected_tag_name,
                "\"{actual_tag_name}\" != \"{expected_tag_name}\""
            );
        }
    }

    #[tokio::test]
    async fn notes_list() {
        let client = Client::new();
        let result = client
            .list_notes()
            .execute()
            .await
            .expect("failed to list notes");
        assert!(!result.notes.is_empty());
        dbg!(result);
    }

    #[tokio::test]
    async fn source() {
        let client = Client::new();

        let response_1 = client
            .list_posts()
            .id(NonZeroU64::new(1))
            .execute()
            .await
            .expect("failed to get post 1");
        let post_1 = response_1.posts.first().expect("missing post");
        assert!(post_1.id.get() == 1);
        assert!(post_1.source.is_none());

        let response_3 = client
            .list_posts()
            .id(NonZeroU64::new(3))
            .execute()
            .await
            .expect("failed to get post 3");
        let post_3 = response_3.posts.first().expect("missing post");
        assert!(post_3.id.get() == 3);
        assert!(post_3.source.as_deref() == Some("https://www.pixiv.net/en/artworks/12972758"));
    }
}
