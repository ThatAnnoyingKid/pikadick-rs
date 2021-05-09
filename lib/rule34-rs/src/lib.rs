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
pub use scraper::Html;

/// A helper to build a search query.
#[derive(Debug)]
pub struct SearchQueryBuilder(String);

impl SearchQueryBuilder {
    /// Make a new [`SearchQueryBuilder`].
    pub fn new() -> Self {
        SearchQueryBuilder(String::new())
    }

    /// Add a tag. Spaces are replaced with underscores, so this can only be one tag.
    pub fn add_tag(&mut self, tag: &str) -> &mut Self {
        self.0.reserve(tag.len());
        for c in tag.chars() {
            if c == ' ' {
                self.0.push('_');
            } else {
                self.0.push(c);
            }
        }
        self.0.push(' ');

        self
    }

    /// Call [`SearchQueryBuilder::add_tag`] on each element of the given iterator.
    pub fn add_tag_iter<I, S>(&mut self, iter: I) -> &mut Self
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
    {
        for s in iter {
            self.add_tag(s.as_ref());
        }

        self
    }

    /// Take the built query string out, resetting this builder's state.
    pub fn take_query_string(&mut self) -> String {
        if self.0.ends_with(' ') {
            self.0.pop();
        }

        std::mem::take(&mut self.0)
    }

    /// Convert into a usable query string.
    pub fn into_query_string(mut self) -> String {
        if self.0.ends_with(' ') {
            self.0.pop();
        }

        self.0
    }
}

impl From<SearchQueryBuilder> for String {
    fn from(search_query_builder: SearchQueryBuilder) -> Self {
        search_query_builder.into_query_string()
    }
}

impl Default for SearchQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_search_query_works() {
        let query = SearchQueryBuilder::new()
            .add_tag("deep space waifu")
            .take_query_string();
        assert_eq!(query, "deep_space_waifu");
    }
}
