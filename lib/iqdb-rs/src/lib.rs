mod types;

use crate::types::SearchResults;
use scraper::Html;

// /// The max file size in bytes
// const MAX_FILE_SIZE: usize = 8_388_608;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Reqwest Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A tokio task failed to join
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),

    /// Invalid Search Results
    #[error("invalid search results")]
    InvalidSearchResults(#[from] crate::types::search_results::FromHtmlError),
}

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Look up an image.
    pub async fn search(&self, url: &str) -> Result<SearchResults, Error> {
        let form = reqwest::multipart::Form::new().text("url", url.to_string());
        let text = self
            .client
            .post("http://iqdb.org/")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let results = tokio::task::spawn_blocking(move || {
            let html = Html::parse_document(&text);
            SearchResults::from_html(&html)
        })
        .await??;

        Ok(results)
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
        let url = "https://konachan.com/jpeg/4db69f9f17b811561b32f1487540e12e/Konachan.com%20-%20162973%20aya_%28star%29%20brown_hair%20grass%20night%20original%20scenic%20school_uniform%20sky%20stars.jpg";
        let result = client.search(url).await.expect("failed to search");

        dbg!(result);
    }
}
