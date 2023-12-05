use crate::{
    Client,
    Error,
    TagList,
};
use url::Url;

/// A query builder to get tags
#[derive(Debug)]
pub struct TagListQueryBuilder<'a> {
    /// The id
    pub id: Option<u64>,

    /// The max number of tags to return.
    ///
    /// This looks like it is capped at 1000, requests for more only return 1000.
    /// This behavior is undocumented however.
    /// As such, requesting more than 1000 does not trigger an error.
    pub limit: Option<u16>,

    /// The page id
    ///
    /// This returns the page of the given number, starting at 0.
    /// This option is undocumented.
    pub pid: Option<u64>,

    /// The tag name to look up
    ///
    /// This is a single tag name.
    /// This option is undocumented
    pub name: Option<&'a str>,

    /// The name pattern to look up using a SQL LIKE clause.
    ///
    /// % = multi char wildcard
    /// _ = single char wildcard
    /// This option is undocumented.
    pub name_pattern: Option<&'a str>,

    /// The field to order results by.
    ///
    /// name: Order by tag name
    /// count: Order by tag count
    pub order: Option<&'a str>,

    /// The client
    client: &'a Client,
}

impl<'a> TagListQueryBuilder<'a> {
    /// Make a new [`TagsListQueryBuilder`]
    pub fn new(client: &'a Client) -> Self {
        Self {
            id: None,
            limit: None,
            pid: None,
            name: None,
            name_pattern: None,
            order: None,

            client,
        }
    }

    /// Set the tag id
    pub fn id(&mut self, id: Option<u64>) -> &mut Self {
        self.id = id;
        self
    }

    /// Set the limit.
    ///
    /// This looks like it is capped at 1000, requests for more only return 1000.
    /// This behavior is undocumented however.
    /// As such, requesting more than 1000 does not trigger an error.
    pub fn limit(&mut self, limit: Option<u16>) -> &mut Self {
        self.limit = limit;
        self
    }

    /// Set the page id
    ///
    /// This returns the page of the given number, starting at 0.
    /// This option is undocumented.
    pub fn pid(&mut self, pid: Option<u64>) -> &mut Self {
        self.pid = pid;
        self
    }

    /// The tag name to look up
    ///
    /// This is a single tag name.
    /// This option is undocumented
    pub fn name(&'a mut self, name: Option<&'a str>) -> &'a mut Self {
        self.name = name;
        self
    }

    /// The name pattern to look up using a SQL LIKE clause.
    ///
    /// % = multi char wildcard
    /// _ = single char wildcard
    /// This option is undocumented.
    pub fn name_pattern(&'a mut self, name_pattern: Option<&'a str>) -> &'a mut Self {
        self.name_pattern = name_pattern;
        self
    }

    /// The field to order results by.
    ///
    /// name: Order by tag name
    /// count: Order by tag count
    /// This option is undocumented.
    pub fn order(&'a mut self, order: Option<&'a str>) -> &'a mut Self {
        self.order = order;
        self
    }

    /// Get the url for this query.
    pub fn get_url(&self) -> Result<Url, Error> {
        let mut url = Url::parse_with_params(
            crate::API_BASE_URL,
            &[("page", "dapi"), ("s", "tag"), ("q", "index")],
        )?;

        {
            let mut query_pairs = url.query_pairs_mut();

            if let Some(id) = self.id {
                let mut id_buffer = itoa::Buffer::new();
                query_pairs.append_pair("id", id_buffer.format(id));
            }

            if let Some(limit) = self.limit {
                let mut limit_buffer = itoa::Buffer::new();
                query_pairs.append_pair("limit", limit_buffer.format(limit));
            }

            if let Some(pid) = self.pid {
                let mut pid_buffer = itoa::Buffer::new();
                query_pairs.append_pair("pid", pid_buffer.format(pid));
            }

            if let Some(name) = self.name {
                query_pairs.append_pair("name", name);
            }

            if let Some(name_pattern) = self.name_pattern {
                query_pairs.append_pair("name_pattern", name_pattern);
            }

            if let Some(order) = self.order {
                query_pairs.append_pair("order", order);
            }
        }

        Ok(url)
    }

    /// Execute the query
    pub async fn execute(&self) -> Result<TagList, Error> {
        let url = self.get_url()?;

        // We run this on the blocking threadpool out of an abundance of caution.
        // On a 10th gen i7, this runs around 2.5 milliseconds tops in release mode.
        // However, we won't always run on an i7. This should also work on ARM.
        // Therefore, we choose to punt it to the threadpool.
        //
        // quick_xml appears to be inefficient when using serde.
        // One of the first PRs to fix this is https://github.com/tafia/quick-xml/pull/312.
        // When this lands, we can re-investigate optimizing the serde impl.
        //
        // quick_xml also may get async support via https://github.com/tafia/quick-xml/pull/314 in the future as well,
        // making all this optimizing a moot point.
        self.client.get_xml(url.as_str()).await
    }
}
