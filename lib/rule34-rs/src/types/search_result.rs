use select::{
    document::Document,
    node::Node,
    predicate::{
        Class,
        Name,
    },
};
use url::Url;

/// Results for a search
#[derive(Debug)]
pub struct SearchResult {
    /// Search result entries
    pub entries: Vec<Option<SearchEntry>>,
}

impl SearchResult {
    /// Try to make a SearchResult from a Document
    pub fn from_doc(doc: &Document) -> Result<Self, FromDocError> {
        let content_div = doc
            .find(Class("content"))
            .last()
            .ok_or(FromDocError::MissingContentDiv)?;

        let entries = content_div
            .find(Name("span"))
            .map(SearchEntry::from_node)
            .collect();

        Ok(SearchResult { entries })
    }
}

/// Error that may occur while making a SearchResult from a Document
#[derive(Debug, thiserror::Error)]
pub enum FromDocError {
    /// Missing Content Div
    #[error("missing content div")]
    MissingContentDiv,
}

/// Search Result Entry
#[derive(Debug)]
pub struct SearchEntry {
    /// Entry ID
    pub id: u64,
    /// Entry Url
    pub link: Url,
    /// Thumbnail URL
    pub thumb: Url,
    /// Description
    pub desc: String,
}

impl SearchEntry {
    /// Try to make a SearchEntry from a Node
    pub fn from_node(n: Node) -> Option<SearchEntry> {
        let id_str = n.attr("id")?.trim_start_matches('s');
        let id = id_str.parse().ok()?;
        let link = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=view",
            &[("id", id_str)],
        )
        .ok()?;

        let img = n.find(Name("img")).last()?;
        let thumb = Url::parse(img.attr("src")?).ok()?;
        let desc = String::from(img.attr("alt")?.trim());

        Some(SearchEntry {
            id,
            link,
            thumb,
            desc,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const GIF_DOC_STR: &str = include_str!("../../test_data/gif_search.html");

    #[test]
    fn from_doc_gif() {
        let doc = Document::from(GIF_DOC_STR);
        SearchResult::from_doc(&doc).unwrap();
    }
}
