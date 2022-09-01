use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{
    Html,
    Selector,
};

static SCRIPT_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("script").expect("invalid `SCRIPT_SELECTOR`"));

/// An error that may occur while parsing from html
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Missing media id
    #[error("missing media id")]
    MissingMediaId,

    /// Invalid Media Id
    #[error("invalid media id")]
    InvalidMediaId(std::num::ParseIntError),
}

/// A Post Page
#[derive(Debug)]
pub struct PostPage {
    /// The media id
    pub media_id: u64,
}

impl PostPage {
    /// Parse this from html
    pub(crate) fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        static MEDIA_ID_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new("\"media_id\":\"(\\d*)\"").expect("invalid `MEDIA_ID_REGEX`"));

        let media_id = html
            .select(&SCRIPT_SELECTOR)
            .filter_map(|el| el.text().next())
            .find_map(|text| MEDIA_ID_REGEX.captures(text)?.get(1))
            .ok_or(FromHtmlError::MissingMediaId)?
            .as_str()
            .parse()
            .map_err(FromHtmlError::InvalidMediaId)?;

        Ok(Self { media_id })
    }
}
