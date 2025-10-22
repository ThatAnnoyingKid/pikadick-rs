use crate::{
    Client,
    Error,
    NoteList,
};
use std::num::NonZeroU64;
use url::Url;

/// A query builder to get notes.
///
/// This is undocumented.
#[derive(Debug)]
pub struct NotesListQueryBuilder<'a> {
    /// The post id to get notes for.
    ///
    /// This is undocumented.
    pub post_id: Option<NonZeroU64>,

    /// The client
    client: &'a Client,
}

impl<'a> NotesListQueryBuilder<'a> {
    /// Make a new [`NotesListQueryBuilder`]
    pub fn new(client: &'a Client) -> Self {
        Self {
            post_id: None,

            client,
        }
    }

    /// Set the post id to get notes for.
    ///
    /// This is undocumented.
    pub fn post_id(&mut self, post_id: Option<NonZeroU64>) -> &mut Self {
        self.post_id = post_id;
        self
    }

    /// Get the url for this query.
    pub fn get_url(&self) -> Result<Url, Error> {
        let mut url = Url::parse_with_params(
            crate::API_BASE_URL,
            &[("page", "dapi"), ("s", "note"), ("q", "index")],
        )?;

        {
            let mut query_pairs_mut = url.query_pairs_mut();

            let auth = self.client.get_auth();
            let auth = auth.as_ref().ok_or(Error::MissingAuth)?;
            query_pairs_mut.append_pair("user_id", itoa::Buffer::new().format(auth.user_id));
            query_pairs_mut.append_pair("api_key", &auth.api_key);

            if let Some(post_id) = self.post_id {
                let mut buffer = itoa::Buffer::new();
                query_pairs_mut.append_pair("post_id", buffer.format(post_id.get()));
            }
        }

        Ok(url)
    }

    /// Execute the query
    pub async fn execute(&self) -> Result<NoteList, Error> {
        let url = self.get_url()?;

        self.client.get_xml(url.as_str()).await
    }
}
