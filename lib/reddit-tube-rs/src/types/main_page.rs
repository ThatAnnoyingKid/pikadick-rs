use scraper::{
    Html,
    Selector,
};

/// Error that may occur while parsing a [`MainPage`] from [`Html`].
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    /// missing download form
    #[error("missing download form")]
    MissingDownloadForm,

    /// Missing csrf data
    #[error("missing csrf data")]
    MissingCsrf,
}

/// The main page
#[derive(Debug)]
pub struct MainPage {
    /// The csrf key
    pub csrf_key: String,

    /// The csrf value
    pub csrf_value: String,
}

impl MainPage {
    /// Make a [`MainPage`] from [`Html`].
    pub(crate) fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        let download_form_selector =
            Selector::parse("#download-form").expect("invalid download form selector");
        let download_form = html
            .select(&download_form_selector)
            .next()
            .ok_or(FromHtmlError::MissingDownloadForm)?;

        let csrf_selector = Selector::parse("[name][value]").expect("invalid csrf selector");
        let (csrf_key, csrf_value) = download_form
            .select(&csrf_selector)
            .filter_map(|element| {
                let value = element.value();
                Some((value.attr("name")?, value.attr("value")?))
            })
            .find(|(name, _)| name != &"url")
            .ok_or(FromHtmlError::MissingCsrf)?;

        Ok(MainPage {
            csrf_key: csrf_key.to_string(),
            csrf_value: csrf_value.to_string(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const SAMPLE_1: &str = include_str!("../../test_data/main_page.html");

    #[test]
    fn parse() {
        let html = Html::parse_document(SAMPLE_1);
        let page = MainPage::from_html(&html).expect("failed to parse main page sample 1");
        dbg!(page);
    }
}
