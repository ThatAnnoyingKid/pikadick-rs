use crate::{
    GetVideoResponse,
    MainPage,
    TubeError,
    TubeResult,
};
use scraper::Html;

/// Client
#[derive(Clone, Debug)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Makes a new [`Client`].
    ///
    /// # Panics
    /// Panics if the [`Client`] could not be created.
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
    pub async fn get_main_page(&self) -> TubeResult<MainPage> {
        let res = self.client.get("https://www.reddit.tube/").send().await?;

        let status = res.status();
        if !status.is_success() {
            return Err(TubeError::InvalidStatus(status));
        }

        let body = res.text().await?;

        Ok(tokio::task::spawn_blocking(move || {
            let html = Html::parse_document(body.as_str());
            MainPage::from_html(&html)
        })
        .await??)
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
