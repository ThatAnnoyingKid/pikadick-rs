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
