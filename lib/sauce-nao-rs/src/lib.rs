/// Client type
pub mod client;
/// Image type
pub mod image;
/// Api types
pub mod types;

pub use self::{
    client::Client,
    image::Image,
    types::SearchJson,
};

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
