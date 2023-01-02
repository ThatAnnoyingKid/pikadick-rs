mod client;
mod error;
mod search_query_builder;
mod types;

pub use crate::{
    client::Client,
    error::Error,
    search_query_builder::SearchQueryBuilder,
    types::{
        DeletedImageList,
        HtmlPost,
        Post,
        PostList,
        Tag,
        TagKind,
        TagList,
    },
};
pub use scraper::Html;
pub use url::Url;

/// The maximum number of responses per post list request
pub const POST_LIST_LIMIT_MAX: u16 = 1_000;
/// The maximum number of responses per tags list request
pub const TAGS_LIST_LIMIT_MAX: u16 = 1_000;

// URL constants
const URL_INDEX: &str = "https://rule34.xxx/index.php";

/// Turn a post id into a post url
fn post_id_to_html_post_url(id: u64) -> Url {
    // It shouldn't be possible to make this function fail for any valid id.
    Url::parse_with_params(
        crate::URL_INDEX,
        &[
            ("id", itoa::Buffer::new().format(id)),
            ("page", "post"),
            ("s", "view"),
        ],
    )
    .unwrap()
}
