mod client;
mod error;
mod search_query_builder;
mod types;

pub use crate::{
    client::Client,
    error::Error,
    search_query_builder::SearchQueryBuilder,
    types::{
        DeletedImagesList,
        Post,
        PostListResult,
        Tag,
        TagKind,
        TagsList,
    },
};
pub use scraper::Html;
pub use url::Url;

pub const POST_LIST_LIMIT_MAX: u16 = 1_000;
pub const TAGS_LIST_LIMIT_MAX: u16 = 1_000;

// Default Header values
const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4514.0 Safari/537.36";
const REFERER_STR: &str = "https://rule34.xxx/";
const ACCEPT_LANGUAGE_STR: &str = "en,en-US;q=0,5";
const ACCEPT_STR: &str = "*/*";

// URL constants
const URL_INDEX: &str = "https://rule34.xxx/index.php";

/// Turn a post id into a post url
fn post_id_to_post_url(id: u64) -> Url {
    // It shouldn't be possible to make this function fail for any valid id.
    Url::parse_with_params(
        crate::URL_INDEX,
        &[
            ("id", itoa::Buffer::new().format(id)),
            ("page", "post"),
            ("s", "view"),
        ],
    )
    .expect("failed to turn post id into post url")
}
