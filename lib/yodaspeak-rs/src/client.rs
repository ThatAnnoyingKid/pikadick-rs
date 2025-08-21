use crate::Error;
use reqwest::header::{
    HeaderMap,
    HeaderValue,
    ACCEPT,
};
use scraper::{
    Html,
    Selector,
};
use std::sync::LazyLock;

static ACCEPT_VALUE: HeaderValue = HeaderValue::from_static("*/*");

/// The client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, ACCEPT_VALUE.clone());

        let client = reqwest::Client::builder()
            .user_agent("yodaspeak-rs")
            .default_headers(headers)
            .build()
            .expect("failed to build client");

        Self { client }
    }

    /// Translate an input.
    pub async fn translate(&self, data: &str) -> Result<String, Error> {
        let text = self
            .client
            .post("https://www.yodaspeak.co.uk/index.php")
            .form(&[("YodaMe", data), ("go", "Convert to Yoda-Speak!")])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        tokio::task::spawn_blocking(move || {
            static SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
                Selector::parse("#result textarea").expect("failed to parse `SELECTOR`")
            });
            let html = Html::parse_document(&text);

            let result = html
                .select(&SELECTOR)
                .next()
                .and_then(|el| el.text().next())
                .ok_or(Error::MissingResult)?;

            Ok(result.to_string())
        })
        .await?
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
