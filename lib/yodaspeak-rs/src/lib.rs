use once_cell::sync::Lazy;
use scraper::{
    Html,
    Selector,
};

/// The error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// The translate result is missing
    #[error("missing result")]
    MissingResult,
}

/// The client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Translate an input.
    pub async fn translate(&self, data: &str) -> Result<String, Error> {
        let text = self
            .client
            .post("http://www.yodaspeak.co.uk/index.php")
            .form(&[("YodaMe", data), ("go", "Convert to Yoda-Speak!")])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        tokio::task::spawn_blocking(move || {
            static SELECTOR: Lazy<Selector> = Lazy::new(|| {
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let input = "this translator works";
        let translated = client.translate(input).await.expect("failed to translate");
        dbg!(translated);
    }
}
