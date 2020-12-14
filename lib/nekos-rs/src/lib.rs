mod client;
mod types;

pub use crate::{
    client::Client,
    types::Image,
};
pub use url::Url;

/// Nekos result type
pub type NekosResult<T> = Result<T, NekosError>;

/// Nekos lib error
#[derive(Debug)]
pub enum NekosError {
    /// Reqwest HTTP Error
    Reqwest(reqwest::Error),
    /// Invalid HTTP Status
    InvalidStatus(reqwest::StatusCode),
    /// Invalid JSON
    Json(serde_json::Error),
    /// Invalid URL
    InvalidUrl(url::ParseError),
    /// Io Error
    Io(std::io::Error),
}

impl std::fmt::Display for NekosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NekosError::Reqwest(e) => e.fmt(f),
            NekosError::Json(e) => e.fmt(f),
            NekosError::InvalidStatus(status) => write!(f, "Invalid Status {}", status),
            NekosError::InvalidUrl(e) => e.fmt(f),
            NekosError::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for NekosError {}

impl From<reqwest::Error> for NekosError {
    fn from(e: reqwest::Error) -> Self {
        NekosError::Reqwest(e)
    }
}

impl From<serde_json::Error> for NekosError {
    fn from(e: serde_json::Error) -> NekosError {
        NekosError::Json(e)
    }
}

impl From<std::io::Error> for NekosError {
    fn from(e: std::io::Error) -> NekosError {
        NekosError::Io(e)
    }
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
}
