mod client;
mod error;
mod search_query_builder;
mod types;

pub use crate::{
    client::Client,
    error::RuleError,
    search_query_builder::SearchQueryBuilder,
    types::{
        Post,
        SearchResult,
    },
};
pub use scraper::Html;

// Default Header values
const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4514.0 Safari/537.36";
const REFERER_STR: &str = "https://rule34.xxx/";
const ACCEPT_LANGUAGE_STR: &str = "en,en-US;q=0,5";
const ACCEPT_STR: &str = "*/*";

const URL_INDEX: &str = "https://rule34.xxx/index.php";
