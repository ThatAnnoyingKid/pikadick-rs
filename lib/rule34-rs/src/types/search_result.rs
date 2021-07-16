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
        lazy_static::lazy_static! {
            static ref CONTENT_DIV_SELECTOR: Selector = Selector::parse(".content").expect("invalid content div selector");
            static ref SPAN_SELECTOR: Selector = Selector::parse("span").expect("invalid span selector");
        }

        let content_div = html
            .select(&CONTENT_DIV_SELECTOR)
            .next()
            .ok_or(FromHtmlError::MissingContentDiv)?;

        let entries = content_div
            .select(&SPAN_SELECTOR)
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

    /// Invalid Thumbnail Url
    #[error("invalid thumbnail")]
    InvalidThumbnailUrl(url::ParseError),
}

/// Search Result Entry
#[derive(Debug)]
pub struct SearchEntry {
    /// The post id
    pub id: u64,

    /// The thumbnail url
    pub thumbnail: Url,

    /// The description
    pub description: String,
}

impl SearchEntry {
    /// Try to make a [`SearchEntry`] from an [`ElementRef`]
    pub fn from_element(element: ElementRef) -> Result<SearchEntry, FromElementError> {
        lazy_static::lazy_static! {
            static ref IMG_SELECTOR: Selector = Selector::parse("img").expect("invalid img selector");
        }

        let id = element
            .value()
            .attr("id")
            .ok_or(FromElementError::MissingAttribute("element", "id"))?
            .trim_start_matches('s')
            .parse()
            .map_err(FromElementError::InvalidId)?;

        let img = element
            .select(&IMG_SELECTOR)
            .last()
            .ok_or(FromElementError::MissingElement("img"))?;

        let thumbnail = img
            .value()
            .attr("src")
            .map(Url::parse)
            .ok_or(FromElementError::MissingAttribute("img", "src"))?
            .map_err(FromElementError::InvalidThumbnailUrl)?;

        let description = img
            .value()
            .attr("alt")
            .ok_or(FromElementError::MissingAttribute("img", "alt"))?
            .trim()
            .to_string();

        Ok(SearchEntry {
            id,
            thumbnail,
            description,
        })
    }

    /// Get the post url for this search entry.
    ///
    /// This allocates, so cache the result.
    pub fn get_post_url(&self) -> Url {
        crate::post_id_to_post_url(self.id)
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
