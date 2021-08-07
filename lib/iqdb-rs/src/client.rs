use crate::{
    Error,
    Image,
    SearchResults,
    MAX_FILE_SIZE,
};
use scraper::Html;

#[derive(Clone, Debug)]
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
    pub async fn search(&self, image: impl Into<Image>) -> Result<SearchResults, Error> {
        let mut form =
            reqwest::multipart::Form::new().text("MAX_FILE_SIZE", MAX_FILE_SIZE.to_string());

        match image.into() {
            Image::Url(url) => {
                form = form.text("url", url);
            }
            Image::File { name, body } => {
                let part = reqwest::multipart::Part::stream(body).file_name(name);
                form = form.part("file", part);
            }
        }

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
