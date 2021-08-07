mod types;

pub use crate::types::SearchResults;
pub use reqwest::Body;
use scraper::Html;
use std::{
    borrow::Cow,
    path::Path,
};
pub use tokio_util::codec::{
    BytesCodec,
    FramedRead,
};

/// The max file size in bytes
const MAX_FILE_SIZE: usize = 8_388_608;

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

/// An Image
pub enum Image {
    /// A url to an image
    Url(String),

    /// An image file
    File { name: String, body: Body },
}

impl Image {
    /// Make an [`Image`] from a path, opening the file asynchronously.
    pub async fn from_path(path: &Path) -> std::io::Result<Self> {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or(Cow::Borrowed("file.png"))
            .into();
        let file = tokio::fs::File::open(path).await?;
        Self::from_file(name, file)
    }

    /// Make an [`Image`] from a file and a name.
    pub fn from_file(name: String, file: tokio::fs::File) -> std::io::Result<Self> {
        // What a horrible, horrible, horrible interface...
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        Ok(Self::File { name, body })
    }
}

impl From<String> for Image {
    fn from(url: String) -> Self {
        Image::Url(url)
    }
}

impl From<&str> for Image {
    fn from(url: &str) -> Self {
        Image::Url(url.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const IMAGE_PATH: &str = "./test_data/image.jpeg";

    #[tokio::test]
    async fn it_works_url() {
        let client = Client::new();
        let url = "https://konachan.com/jpeg/4db69f9f17b811561b32f1487540e12e/Konachan.com%20-%20162973%20aya_%28star%29%20brown_hair%20grass%20night%20original%20scenic%20school_uniform%20sky%20stars.jpg";
        let result = client.search(url).await.expect("failed to search");

        dbg!(result);
    }

    #[tokio::test]
    async fn it_works_path() {
        let client = Client::new();
        let path = Path::new(IMAGE_PATH);
        let image = Image::from_path(path).await.expect("failed to open image");
        let result = client.search(image).await.expect("failed to search");

        dbg!(result);
    }
}
