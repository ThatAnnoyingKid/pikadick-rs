use crate::{
    types::{
        ListResult,
        Post,
    },
    DeletedImagesList,
    Error,
    DELETED_IMAGES_ENDPOINT,
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

    /// Create a builder to list posts from rule34.
    pub fn list<'a, 'b>(&'a self) -> ListQueryBuilder<'a, 'b> {
        ListQueryBuilder::new(self)
    }

    /// Get a [`Post`] by `id`.
    pub async fn get_post(&self, id: u64) -> Result<Post, Error> {
        let url = crate::post_id_to_post_url(id);
        let ret = self
            .get_html(url.as_str(), |html| Post::from_html(&html))
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
        let mut url = Url::parse(DELETED_IMAGES_ENDPOINT).expect("invalid DELETED_IMAGES_ENDPOINT");
        if let Some(last_id) = last_id {
            let mut last_id_buf = itoa::Buffer::new();
            url.query_pairs_mut()
                .append_pair("last_id", last_id_buf.format(last_id));
        }
        let text = self.get_text(url.as_str()).await?;
        // Parse on a threadpool since the full returned string is currently around 30 megabytes in size,
        // and we need to run in under a few milliseconds.
        // We need to buffer this all in memory though, since `quick_xml` does not provide a streaming api.
        tokio::task::spawn_blocking(move || {
            let data: DeletedImagesList = quick_xml::de::from_str(&text)?;
            Ok(data)
        })
        .await?
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

const LIMIT_MAX: u16 = 1_000;

/// A builder for list api queries
#[derive(Debug)]
pub struct ListQueryBuilder<'a, 'b> {
    /// The tags
    pub tags: Option<&'b str>,
    /// The page #
    pub pid: Option<u64>,
    /// The post id
    pub id: Option<u64>,
    /// The limit
    pub limit: Option<u16>,

    client: &'a Client,
}

impl<'a, 'b> ListQueryBuilder<'a, 'b> {
    /// Make a new [`ListQueryBuilder`].
    pub fn new(client: &'a Client) -> Self {
        Self {
            tags: None,
            pid: None,
            id: None,
            limit: None,

            client,
        }
    }

    /// Set the tags to list for.
    ///
    /// Querys are based on "tags".
    /// Tags are seperated by spaces, while words are seperated by underscores.
    /// Characters are automatically url-encoded.
    pub fn tags(&mut self, tags: Option<&'b str>) -> &mut Self {
        self.tags = tags;
        self
    }

    /// Set the page number
    pub fn pid(&mut self, pid: Option<u64>) -> &mut Self {
        self.pid = pid;
        self
    }

    /// Set the post id
    pub fn id(&mut self, id: Option<u64>) -> &mut Self {
        self.id = id;
        self
    }

    /// Set the post limit.
    ///
    /// This has a hard upper limit of `1000`.
    pub fn limit(&mut self, limit: Option<u16>) -> &mut Self {
        self.limit = limit;
        self
    }

    /// Get the api url.
    ///
    /// # Errors
    /// This fails if the generated url is invalid,
    /// or if `limit` is greater than `1000`.
    pub fn get_url(&self) -> Result<Url, Error> {
        let mut pid_buffer = itoa::Buffer::new();
        let mut id_buffer = itoa::Buffer::new();
        let mut limit_buffer = itoa::Buffer::new();
        let mut url = Url::parse_with_params(
            crate::URL_INDEX,
            &[
                ("page", "dapi"),
                ("s", "post"),
                ("json", "1"),
                ("q", "index"),
            ],
        )?;

        {
            let mut query_pairs_mut = url.query_pairs_mut();

            if let Some(tags) = self.tags {
                query_pairs_mut.append_pair("tags", tags);
            }

            if let Some(pid) = self.pid {
                query_pairs_mut.append_pair("pid", pid_buffer.format(pid));
            }

            if let Some(id) = self.id {
                query_pairs_mut.append_pair("id", id_buffer.format(id));
            }

            if let Some(limit) = self.limit {
                if limit > LIMIT_MAX {
                    return Err(Error::LimitTooLarge(limit));
                }

                query_pairs_mut.append_pair("limit", limit_buffer.format(limit));
            }
        }

        Ok(url)
    }

    /// Execute the api query and get the results.
    ///
    /// # Returns
    /// Returns an empty list if there are no results.
    pub async fn execute(&self) -> Result<Vec<ListResult>, Error> {
        let url = self.get_url()?;

        // The api sends "" on no results, and serde_json dies instead of giving an empty list.
        // Therefore, we need to handle json parsing instead of reqwest.
        let text = self.client.get_text(url.as_str()).await?;
        if text.is_empty() {
            return Ok(Vec::new());
        }

        Ok(serde_json::from_str(&text)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn search() {
        let client = Client::new();
        let res = client
            .list()
            .tags(Some("rust"))
            .execute()
            .await
            .expect("failed to search rule34 for `rust`");
        dbg!(&res);
        assert!(!res.is_empty());
    }

    async fn get_top_post(query: &str) -> Post {
        let client = Client::new();
        let res = client
            .list()
            .tags(Some(query))
            .execute()
            .await
            .unwrap_or_else(|_| panic!("failed to search rule34 for `{}`", query));
        assert!(!res.is_empty());

        let first = res.first().expect("missing first entry");
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

    #[tokio::test]
    async fn deleted_images_list() {
        let client = Client::new();
        let result = client
            .get_deleted_images(Some(500_000)) // Just choose a high-ish post id here and update to keep the download limited
            .await
            .expect("failed to get deleted images");
        dbg!(result);
    }
}
