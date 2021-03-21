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
    pub entries: Vec<SearchEntry>,
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
            .collect::<Result<_, _>>()?;

        Ok(SearchResult { entries })
    }
}

/// Error that may occur while making a [`SearchResult`] from a [`Document`]
#[derive(Debug, thiserror::Error)]
pub enum FromDocError {
    /// Missing Content Div
    #[error("missing content div")]
    MissingContentDiv,

    /// Invalid [`SearchEntry`]
    #[error("invalid search result: {0}")]
    InvalidSearchEntry(#[from] FromNodeError),
}

/// Search Result Entry
#[derive(Debug)]
pub struct SearchEntry {
    /// Entry ID
    pub id: u64,

    /// Entry Url
    pub link: Url,

    /// Thumbnail URL
    pub thumbnail: Url,

    /// Description
    pub description: String,
}

/// The error that may occur if a [`SearchEntry`] could not be parsed from a [`Node`]
#[derive(Debug, thiserror::Error)]
pub enum FromNodeError {
    /// Missing Attribute
    #[error("missing attribute '{1}' in element '{0}'")]
    MissingAttribute(&'static str, &'static str),

    /// Missing Element
    #[error("missing element '{0}'")]
    MissingElement(&'static str),

    #[error("invalid id: '{0}'")]
    InvalidId(std::num::ParseIntError),

    /// Invalid Link Url
    #[error("invalid link: {0}")]
    InvalidLink(url::ParseError),

    /// Invalid Thumbnail Url
    #[error("invalid thumbnail: {0}")]
    InvalidThumbnailUrl(url::ParseError),
}

impl SearchEntry {
    /// Try to make a [`SearchEntry`] from a [`Node`]
    pub fn from_node(node: Node) -> Result<SearchEntry, FromNodeError> {
        let id_str = node
            .attr("id")
            .ok_or(FromNodeError::MissingAttribute("id", "node"))?
            .trim_start_matches('s');
        let id: u64 = id_str.parse().map_err(FromNodeError::InvalidId)?;
        let link = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=view",
            &[("id", id_str)],
        )
        .map_err(FromNodeError::InvalidLink)?;

        let img = node
            .find(Name("img"))
            .last()
            .ok_or(FromNodeError::MissingElement("img"))?;
        let thumbnail = Url::parse(
            img.attr("src")
                .ok_or(FromNodeError::MissingAttribute("src", "img"))?,
        )
        .map_err(FromNodeError::InvalidThumbnailUrl)?;
        let description = img
            .attr("alt")
            .ok_or(FromNodeError::MissingAttribute("alt", "img"))?
            .trim()
            .to_string();

        Ok(SearchEntry {
            id,
            link,
            thumbnail,
            description,
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
