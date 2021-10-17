use url::Url;

/// A list of posts
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PostList {
    /// The # of posts.
    ///
    /// This is the total # of posts, not the # in this list.
    pub count: u64,

    /// The current offset
    pub offset: u64,

    /// The posts
    #[serde(alias = "post")]
    pub posts: Vec<Post>,
}

/// A Post
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Post {
    /// The height
    pub height: u64,

    /// ?
    pub score: u64,

    /// The main file url
    pub file_url: Url,

    /// The parent post id
    pub parent_id: Option<u64>,

    /// The sample url
    pub sample_url: Url,

    /// The sample width
    pub sample_width: u64,

    /// The sample height
    pub sample_height: u64,

    /// The preview url
    pub preview_url: Url,

    /// The image rating
    pub rating: Rating,

    /// Tags
    pub tags: String,

    /// The id the post
    pub id: u64,

    /// image width
    pub width: u64,

    /// ?
    pub change: u64,

    /// ?
    pub md5: String,

    /// The creator id
    pub creator_id: u64,

    /// Whether this has children
    pub has_children: bool,

    /// Creation date
    pub created_at: String,

    /// ?
    pub status: String,

    /// The original source.
    ///
    /// May or may not be a url
    pub source: Option<String>,

    /// ?
    pub has_notes: bool,

    /// ?
    pub has_comments: bool,

    /// preview image width
    pub preview_width: u64,

    /// preview image width
    pub preview_height: u64,
}

impl Post {
    /// Get the html post url for this.
    ///
    /// This allocates, so cache the result.
    pub fn get_html_post_url(&self) -> Url {
        crate::post_id_to_html_post_url(self.id)
    }
}

/// A post rating
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Rating {
    /// Questionable
    #[serde(rename = "q")]
    Questionable,

    /// Explicit
    #[serde(rename = "e")]
    Explicit,

    /// Safe
    #[serde(rename = "s")]
    Safe,
}
