pub mod types;

pub use crate::types::{
    GetVideoResponse,
    MainPage,
};
pub use reqwest::StatusCode;
use select::document::Document;

/// Result Error
pub type TubeResult<T> = Result<T, TubeError>;

/// Client Error
#[derive(Debug, thiserror::Error)]
pub enum TubeError {
    /// HTTP Reqwest Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    #[error("invalid http status '{0}'")]
    InvalidStatus(StatusCode),

    /// Invalid Json
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Invalid main page
    #[error("invalid main page")]
    InvalidMainPage,
}

/// Client
#[derive(Clone, Debug)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Makes a new client
    pub fn new() -> Self {
        Client {
            client: reqwest::ClientBuilder::new()
                .cookie_store(true)
                .build()
                .unwrap(),
        }
    }

    /// Gets MainPage data. Useful only to fetch csrf token and pass it to another api call.
    pub async fn get_main_page(&self) -> TubeResult<MainPage> {
        let res = self.client.get("https://www.reddit.tube/").send().await?;

        let status = res.status();
        if !status.is_success() {
            return Err(TubeError::InvalidStatus(status));
        }

        let body = res.text().await?;
        let doc = Document::from(body.as_str());

        MainPage::from_doc(&doc).ok_or(TubeError::InvalidMainPage)
    }

    /// main_page is exposed publicly as the same main_page may be used for multiple get_video calls as long as they are close together chronologically,
    /// most likely at least a few seconds or minutes
    /// calling mainpage will also aquire a new session cookie if necessary, so make sure to call get_main_page to refresh the csrf token if it expires
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

        Ok(serde_json::from_str(&body)?)
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
