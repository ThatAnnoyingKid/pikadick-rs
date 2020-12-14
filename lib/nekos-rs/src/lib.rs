mod client;
mod types;

pub use crate::{
    client::Client,
    types::{
        Image,
        ImageList,
    },
};
pub use url::Url;

/// Nekos result type
pub type NekosResult<T> = Result<T, NekosError>;

/// Nekos lib error
#[derive(Debug, thiserror::Error)]
pub enum NekosError {
    /// Reqwest HTTP Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    /// Invalid HTTP Status
    #[error("invalid status {0}")]
    InvalidStatus(reqwest::StatusCode),
    /// Invalid JSON
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// Invalid URL
    #[error("{0}")]
    InvalidUrl(#[from] url::ParseError),
    /// Io Error
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let image_list = client.get_random(Some(false), 10).await.unwrap();

        assert_eq!(image_list.images.len(), 10);

        let image_url = image_list.images[0].get_url().unwrap();
        let mut image = Vec::new();
        client.copy_res_to(&image_url, &mut image).await.unwrap();
    }

    #[tokio::test]
    async fn get_nsfw() {
        let client = Client::new();
        let image_list = client.get_random(Some(true), 10).await.unwrap();
        assert_eq!(image_list.images.len(), 10);
    }

    #[tokio::test]
    async fn get_non_nsfw() {
        let client = Client::new();
        let image_list = client.get_random(Some(false), 10).await.unwrap();
        assert_eq!(image_list.images.len(), 10);
    }

    #[tokio::test]
    async fn get_100() {
        let client = Client::new();
        let image_list = client.get_random(None, 100).await.unwrap();
        assert_eq!(image_list.images.len(), 100);
    }
}
