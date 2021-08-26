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

/// Nekos lib error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// Invalid URL
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),
    /// Io Error
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let image_list = client
            .get_random(Some(false), 10)
            .await
            .expect("failed to get random");

        assert_eq!(image_list.images.len(), 10);

        let image_url = image_list.images[0]
            .get_url()
            .expect("missing first element");
        let mut image = Vec::new();
        client
            .copy_res_to(image_url.as_str(), &mut image)
            .await
            .expect("failed to download");
    }

    #[tokio::test]
    async fn get_nsfw() {
        let client = Client::new();
        let image_list = client
            .get_random(Some(true), 10)
            .await
            .expect("failed to get random");
        assert_eq!(image_list.images.len(), 10);
    }

    #[tokio::test]
    async fn get_non_nsfw() {
        let client = Client::new();
        let image_list = client
            .get_random(Some(false), 10)
            .await
            .expect("failed to get random");
        assert_eq!(image_list.images.len(), 10);
    }

    #[tokio::test]
    async fn get_100() {
        let client = Client::new();
        let image_list = client
            .get_random(None, 100)
            .await
            .expect("failed to get 100");
        assert_eq!(image_list.images.len(), 100);
    }
}
