mod client;
mod error;
mod search_query_builder;
mod types;
mod util;

#[cfg(feature = "scrape")]
pub use crate::types::HtmlPost;
pub use crate::{
    client::{
        Client,
        NotesListQueryBuilder,
        PostListQueryBuilder,
        TagListQueryBuilder,
    },
    error::Error,
    search_query_builder::SearchQueryBuilder,
    types::{
        DeletedImageList,
        Note,
        NoteList,
        Post,
        PostList,
        PostStatus,
        Rating,
        Tag,
        TagKind,
        TagList,
    },
};
#[cfg(feature = "scrape")]
pub use scraper::Html;
use std::num::NonZeroU64;
pub use url::Url;

/// The maximum number of responses per post list request
pub const POST_LIST_LIMIT_MAX: u16 = 1_000;
/// The maximum number of responses per tags list request.
///
/// This is undocumented.
/// The documented limit is 100.
pub const TAGS_LIST_LIMIT_MAX: u16 = 1_000;

// URL constants
pub(crate) const URL_INDEX: &str = "https://rule34.xxx/index.php";

/// The base Api Url
pub(crate) const API_BASE_URL: &str = "https://api.rule34.xxx/index.php";

/// Turn a post id into a post url
fn post_id_to_html_post_url(id: NonZeroU64) -> Url {
    // It shouldn't be possible to make this function fail for any valid id.
    Url::parse_with_params(
        crate::URL_INDEX,
        &[
            ("id", itoa::Buffer::new().format(id.get())),
            ("page", "post"),
            ("s", "view"),
        ],
    )
    .unwrap()
}
