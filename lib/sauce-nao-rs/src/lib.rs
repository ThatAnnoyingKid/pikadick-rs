/// Image type
pub mod image;
/// Api types
pub mod types;

pub use self::{
    image::Image,
    types::SearchJson,
};
use std::sync::Arc;
use url::Url;

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A reqwest error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A URL parse error
    #[error("invalid url")]
    Url(#[from] url::ParseError),
}

/// The sauce nao client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: Arc<str>,
}

impl Client {
    /// Create a new [`Client`].
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Arc::from(api_key),
        }
    }

    /// Look up an image
    pub async fn search(&self, image: impl Into<Image>) -> Result<SearchJson, Error> {
        let image = image.into();
        let mut url = Url::parse_with_params(
            "https://saucenao.com/search.php?output_type=2",
            &[("api_key", &*self.api_key)],
        )?;

        let mut part = None;
        match image {
            Image::Url(image_url) => {
                url.query_pairs_mut().append_pair("url", &image_url);
            }
            Image::File { name, body } => {
                part = Some(reqwest::multipart::Part::stream(body).file_name(name));
            }
        }

        let mut res = self.client.post(url.as_str());
        if let Some(part) = part {
            let form = reqwest::multipart::Form::new().part("file", part);
            res = res.multipart(form);
        }
        Ok(res.send().await?.error_for_status()?.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const API_KEY: &str = include_str!("../api_key.txt");
    const IMAGE_PATH: &str = "./test_data/oZjCxGo.jpg";

    #[tokio::test]
    #[ignore]
    async fn search_url_works() {
        let client = Client::new(API_KEY);
        let results = client
            .search("https://i.imgur.com/oZjCxGo.jpg")
            .await
            .expect("failed to search");
        dbg!(results);
    }

    #[tokio::test]
    #[ignore]
    async fn search_file_works() {
        let image = Image::from_path(IMAGE_PATH.as_ref())
            .await
            .expect("failed to open image");
        let client = Client::new(API_KEY);
        let results = client.search(image).await.expect("failed to search");
        dbg!(results);
    }
}
