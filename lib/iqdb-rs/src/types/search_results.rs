use scraper::{
    element_ref::ElementRef,
    Html,
    Selector,
};
use std::borrow::Cow;
use url::Url;

/// Error that may occur while parsing a [`SearchResults`] from html.
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// Missing the pages div
    #[error("missing pages div")]
    MissingPagesDiv,

    /// Missing Your Image Div
    #[error("missing your image div")]
    MissingYourImageDiv,

    /// Missing best match div
    #[error("missing best match div")]
    MissingBestMatchDiv,

    #[error("failed to parse a match")]
    InvalidMatch(#[from] FromElementError),
}

/// The results of an image search
#[derive(Debug)]
pub struct SearchResults {
    /// The best match
    pub best_match: Option<Match>,

    /// Possible matches
    pub possible_matches: Vec<Match>,
}

impl SearchResults {
    /// Make a [`SearchResults`] from html
    pub fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        lazy_static::lazy_static! {
            static ref PAGES_SELECTOR: Selector = Selector::parse(".pages").expect("invalid pages selector");
            static ref DIV_SELECTOR: Selector = Selector::parse("div").expect("invalid div selector");
        };

        let pages_el = html
            .select(&PAGES_SELECTOR)
            .next()
            .ok_or(FromHtmlError::MissingPagesDiv)?;

        let mut pages_el_divs_iter = pages_el.select(&DIV_SELECTOR);

        let _your_image = pages_el_divs_iter
            .next()
            .ok_or(FromHtmlError::MissingYourImageDiv)?;

        let best_match_div = pages_el_divs_iter
            .next()
            .ok_or(FromHtmlError::MissingBestMatchDiv)?;

        let best_match = if best_match_div.value().classes().any(|el| el == "nomatch") {
            None
        } else {
            Some(Match::from_element(best_match_div)?)
        };

        let possible_matches = pages_el_divs_iter
            .map(Match::from_element)
            .collect::<Result<_, _>>()?;

        Ok(Self {
            best_match,
            possible_matches,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FromElementError {
    /// Missing a link
    #[error("missing link")]
    MissingLink,

    /// A link was missing a href
    #[error("missing href")]
    MissingHref,

    /// A link href was invalid
    #[error("invalid href")]
    InvalidHref(#[source] url::ParseError),

    /// Missing an img element
    #[error("missing img")]
    MissingImg,

    /// Missing img url
    #[error("missing img url")]
    MissingImgUrl,

    /// Invalid img url
    #[error("invalid img url")]
    InvalidImgUrl(#[source] url::ParseError),
}

/// A best or possible image match
#[derive(Debug)]
pub struct Match {
    /// The page url of the match
    pub url: Url,

    /// The url of the img
    pub image_url: Url,
}

impl Match {
    /// Create an element
    pub fn from_element(element: ElementRef<'_>) -> Result<Self, FromElementError> {
        lazy_static::lazy_static! {
            static ref LINK_SELECTOR: Selector = Selector::parse("a").expect("invalid link selector");
            static ref IMG_SELECTOR: Selector = Selector::parse("img").expect("invalid img selector");
        }

        let link_el = element
            .select(&LINK_SELECTOR)
            .next()
            .ok_or(FromElementError::MissingLink)?;

        let link_href = link_el
            .value()
            .attr("href")
            .ok_or(FromElementError::MissingHref)
            .map(fixup_url)?;

        let url = Url::parse(&link_href).map_err(FromElementError::InvalidHref)?;

        let img_el = element
            .select(&IMG_SELECTOR)
            .next()
            .ok_or(FromElementError::MissingImg)?;

        let img_src = img_el
            .value()
            .attr("src")
            .ok_or(FromElementError::MissingImgUrl)
            .map(fixup_url)?;

        let image_url = Url::parse(&img_src).map_err(FromElementError::InvalidImgUrl)?;

        Ok(Self { url, image_url })
    }
}

/// Fixup a url for parsing
fn fixup_url(link: &str) -> Cow<'_, str> {
    let mut link = Cow::Borrowed(link);

    // Fixup no protocol
    if link.starts_with("//") {
        link = format!("https:{}", link).into()
    }

    // Fixup relative urls
    if link.starts_with('/') {
        link = format!("https://iqdb.org{}", link).into();
    }

    link
}

#[cfg(test)]
mod test {
    use super::*;

    const VALID: &str = include_str!("../../test_data/valid.html");
    const INVALID: &str = include_str!("../../test_data/invalid.html");

    #[test]
    fn parse_valid_search_results() {
        let html = Html::parse_document(VALID);

        let results = SearchResults::from_html(&html).expect("failed to parse");
        dbg!(&results);
        assert!(results.best_match.is_some());
    }

    #[test]
    fn parse_invalid_search_results() {
        let html = Html::parse_document(INVALID);

        let results = SearchResults::from_html(&html).expect("failed to parse");
        dbg!(&results);
        assert!(results.best_match.is_none());
    }
}
