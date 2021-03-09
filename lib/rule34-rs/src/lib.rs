mod client;
mod error;
mod types;

pub use crate::{
    client::Client,
    error::{
        RuleError,
        RuleResult,
    },
    types::{
        Post,
        SearchResult,
    },
};
pub use select::document::Document;

/// Utility function to build a search query.
///
/// # Errors
/// Returns `None` if a tag contains an underscore.
///
pub fn build_search_query<I: Iterator<Item = S>, S: AsRef<str>>(tags: I) -> Option<String> {
    let mut ret = String::new();
    for tag in tags {
        let tag = tag.as_ref();
        if tag.contains('_') {
            // A naiive way to let people get what they want. This will likely need to be improved in the future.
            return None;
        }
        ret.push_str(tag);
        ret.push('_');
    }

    ret.pop(); // Remove ending '_'

    Some(ret)
}

#[cfg(test)]
mod test {
    #[test]
    fn build_search_query_works() {
        let query = crate::build_search_query("deep space waifu".split(' ')).unwrap();
        assert_eq!(query, "deep_space_waifu");
    }
}
