mod client;
mod image;
mod types;

pub use crate::{
    client::Client,
    image::Image,
    types::SearchResults,
};
pub use reqwest::Body;
pub use scraper::Html;
pub use tokio_util::codec::{
    BytesCodec,
    FramedRead,
};

/// The max file size in bytes
pub const MAX_FILE_SIZE: usize = 8_388_608;

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

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

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
