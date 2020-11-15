use select::{
    document::Document,
    node::Node,
    predicate::{
        Class,
        Name,
    },
};
use url::Url;

#[derive(Debug)]
pub struct SearchResult {
    pub entries: Vec<Option<SearchEntry>>,
}

impl SearchResult {
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

#[derive(Debug)]
pub enum FromDocError {
    MissingContentDiv,
}

impl std::fmt::Display for FromDocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "missing content div".fmt(f)
    }
}

impl std::error::Error for FromDocError {}

#[derive(Debug)]
pub struct SearchEntry {
    pub id: u64,
    pub link: Url,
    pub thumb: Url,
    pub desc: String,
}

impl SearchEntry {
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
