use scraper::{
    ElementRef,
    Html,
    Selector,
};
use url::Url;

/// Error that may occur while making a [`SearchResult`] from [`Html`]
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Missing Content Div
    #[error("missing content div")]
    MissingContentDiv,

    /// Invalid [`SearchEntry`]
    #[error("invalid search entry")]
    InvalidSearchEntry(#[from] FromElementError),
}

/// Results for a search
#[derive(Debug)]
pub struct SearchResult {
    /// Search result entries
    pub entries: Vec<SearchEntry>,
}

impl SearchResult {
    /// Try to make a [`SearchResult`] from [`Html`].
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let content_div_selector =
            Selector::parse(".content").expect("invalid content div selector");
        let content_div = html
            .select(&content_div_selector)
            .next()
            .ok_or(FromHtmlError::MissingContentDiv)?;

        let span_selector = Selector::parse("span").expect("invalid span selector");
        let entries = content_div
            .select(&span_selector)
            .map(SearchEntry::from_element)
            .collect::<Result<_, _>>()?;

        Ok(SearchResult { entries })
    }
}

/// The error that may occur if a [`SearchEntry`] could not be parsed from an [`ElementRef`]
#[derive(Debug, thiserror::Error)]
pub enum FromElementError {
    /// Missing Attribute
    #[error("missing attribute '{1}' in element '{0}'")]
    MissingAttribute(&'static str, &'static str),

    /// Missing Element
    #[error("missing element '{0}'")]
    MissingElement(&'static str),

    #[error("invalid id")]
    InvalidId(std::num::ParseIntError),

    /// Invalid Link Url
    #[error("invalid link")]
    InvalidLink(url::ParseError),

    /// Invalid Thumbnail Url
    #[error("invalid thumbnail")]
    InvalidThumbnailUrl(url::ParseError),
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

impl SearchEntry {
    /// Try to make a [`SearchEntry`] from an [`ElementRef`]
    pub fn from_element(element: ElementRef) -> Result<SearchEntry, FromElementError> {
        let id_str = element
            .value()
            .attr("id")
            .ok_or(FromElementError::MissingAttribute("element", "id"))?
            .trim_start_matches('s');
        let id: u64 = id_str.parse().map_err(FromElementError::InvalidId)?;

        let link = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=view",
            &[("id", id_str)],
        )
        .map_err(FromElementError::InvalidLink)?;

        let img_selector = Selector::parse("img").expect("invalid img selector");
        let img = element
            .select(&img_selector)
            .last()
            .ok_or(FromElementError::MissingElement("img"))?;

        let thumbnail = img
            .value()
            .attr("src")
            .ok_or(FromElementError::MissingAttribute("img", "src"))?;
        let thumbnail = Url::parse(thumbnail).map_err(FromElementError::InvalidThumbnailUrl)?;

        let description = img
            .value()
            .attr("alt")
            .ok_or(FromElementError::MissingAttribute("img", "alt"))?
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

    const GIF_HTML_STR: &str = include_str!("../../test_data/gif_search.html");

    #[test]
    fn from_gif_html() {
        let html = Html::parse_document(GIF_HTML_STR);
        SearchResult::from_html(&html).expect("invalid gif search result");
    }
}
