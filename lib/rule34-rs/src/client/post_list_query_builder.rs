use crate::{
    Client,
    Error,
    PostList,
};
use std::num::NonZeroU64;
use url::Url;

/// A builder for post list api queries
#[derive(Debug, Copy, Clone)]
pub struct PostListQueryBuilder<'a> {
    /// The tags.
    pub tags: Option<&'a str>,

    /// The page #
    ///
    /// Starts at 0.
    pub pid: Option<u64>,

    /// The post id.
    pub id: Option<NonZeroU64>,

    /// The limit.
    pub limit: Option<u16>,

    /// The client ref.
    client: &'a Client,
}

impl<'a> PostListQueryBuilder<'a> {
    /// Make a new [`PostListQueryBuilder`].
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
    pub fn tags(&mut self, tags: Option<&'a str>) -> &mut Self {
        self.tags = tags;
        self
    }

    /// Set the page number
    pub fn pid(&mut self, pid: Option<u64>) -> &mut Self {
        self.pid = pid;
        self
    }

    /// Set the post id
    pub fn id(&mut self, id: Option<NonZeroU64>) -> &mut Self {
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
    /// This fails if:
    /// 1. The generated url is invalid
    /// 2. `limit` is greater than `1000`
    pub fn get_url(&self) -> Result<Url, Error> {
        let mut pid_buffer = itoa::Buffer::new();
        let mut id_buffer = itoa::Buffer::new();
        let mut limit_buffer = itoa::Buffer::new();

        let mut url = Url::parse_with_params(
            crate::API_BASE_URL,
            &[("page", "dapi"), ("s", "post"), ("q", "index")],
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
                query_pairs_mut.append_pair("id", id_buffer.format(id.get()));
            }

            if let Some(limit) = self.limit {
                if limit > crate::POST_LIST_LIMIT_MAX {
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
    pub async fn execute(&self) -> Result<PostList, Error> {
        let url = self.get_url()?;
        self.client.get_xml(url.as_str()).await
    }
}
