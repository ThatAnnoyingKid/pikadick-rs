pub mod types;

pub use crate::types::{
    GetVideoResponse,
    MainPage,
};
pub use reqwest::StatusCode;
use select::document::Document;

/// Result Type
///
pub type TubeResult<T> = Result<T, TubeError>;

/// Client Error
///
#[derive(Debug, thiserror::Error)]
pub enum TubeError {
    /// HTTP Reqwest Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    ///
    #[error("invalid http status '{0}'")]
    InvalidStatus(StatusCode),

    /// Invalid Json
    ///
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    /// Invalid main page
    ///
    #[error("invalid main page")]
    InvalidMainPage,

    /// A Tokio Task Panicked
    ///
    #[error("{0}")]
    TokioJoin(#[from] tokio::task::JoinError),
}

/// Client
///
#[derive(Clone, Debug)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Makes a new [`Client`].
    ///
    /// # Panics
    /// Panics if the [`Client`] could not be created.
    ///
    pub fn new() -> Self {
        Client {
            client: reqwest::ClientBuilder::new()
                .cookie_store(true)
                .build()
                .expect("valid client"),
        }
    }

    /// Gets [`MainPage`] data.
    ///
    /// Useful only to fetch csrf token and pass it to another api call.
    ///
    /// # Errors
    /// Returns an error if the [`MainPage`] could not be fetched.
    ///
    pub async fn get_main_page(&self) -> TubeResult<MainPage> {
        let res = self.client.get("https://www.reddit.tube/").send().await?;

        let status = res.status();
        if !status.is_success() {
            return Err(TubeError::InvalidStatus(status));
        }

        let body = res.text().await?;

        let main_page = tokio::task::spawn_blocking(move || {
            let doc = Document::from(body.as_str());
            MainPage::from_doc(&doc).ok_or(TubeError::InvalidMainPage)
        })
        .await?;

        main_page
    }

    /// Get a video for a reddit url.
    ///
    /// `main_page` is exposed publicly as the same [`MainPage`] may be used for multiple [`Client::get_video`] calls as long as they are close together chronologically,
    /// most likely at least a few seconds or minutes
    ///
    /// Calling [`Client::get_main_page`] will also aquire a new session cookie if necessary,
    /// so make sure to call get_main_page to refresh the csrf token if it expires
    ///
    /// # Errors
    /// Returns an error if the video url could not be fetched.
    ///
    pub async fn get_video(&self, main_page: &MainPage, url: &str) -> TubeResult<GetVideoResponse> {
        let res = self
            .client
            .post("https://www.reddit.tube/services/get_video")
            .form(&[
                ("url", url),
                ("zip", ""),
                (&main_page.csrf_key, &main_page.csrf_value),
            ])
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            return Err(TubeError::InvalidStatus(status));
        }

        let body = res.text().await?;
        let response = serde_json::from_str(&body)?;

        Ok(response)
    }
}

impl Default for Client {
    fn default() -> Client {
        Client::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let video_url = "https://www.reddit.com/r/dankvideos/comments/h8p0py/pp_removal_time/?utm_source=share&utm_medium=web2x";
        let client = Client::new();
        let main_page = client.get_main_page().await.unwrap();
        let vid = client.get_video(&main_page, video_url).await.unwrap();

        dbg!(vid);
    }
}
