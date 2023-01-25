use crate::{
    types::PostPage,
    Error,
};
use reqwest::header::HeaderMap;
use scraper::Html;

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36";

/// A tiktok client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner HTTP client.
    ///
    /// Should only be used if you want to piggyback off of this for HTTP requests
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`]
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "en-US,en;q=0.8".parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT_STR)
            .cookie_store(false)
            // .use_rustls_tls() // native-tls chokes for some reason
            .default_headers(headers)
            .build()
            .expect("failed to build client");

        Self { client }
    }

    /// Get a page as html and parse it
    async fn get_html<F, T>(&self, url: &str, func: F) -> Result<T, Error>
    where
        F: FnOnce(Html) -> T + Send + 'static,
        T: Send + 'static,
    {
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(tokio::task::spawn_blocking(move || {
            // std::fs::write("out.html", text.as_str());
            let html = Html::parse_document(text.as_str());
            func(html)
        })
        .await?)
    }

    /// Get a tiktok post.
    pub async fn get_post(&self, url: &str) -> Result<PostPage, Error> {
        Ok(self
            .get_html(url, |html| PostPage::from_html(&html))
            .await??)
    }

    /// Get the related item list for a given item id
    pub async fn get_related_item_list(&self, item_id: u64) -> Result<String, Error> {
        let url = "https://us.tiktok.com/api/related/item_list/";

        // This should always create a valid url
        let url = url::Url::parse_with_params(
                url,
                &[
                    ("aid", "1988"),
                    ("app_language", "en"),
                    ("app_name", "tiktok_web"),
                    ("battery_info", "1"),
                    ("browser_language", "en-US"),
                    ("browser_name", "Mozilla"),
                    ("browser_online", "true"),
                    ("browser_platform", "Win32"),
                    ("browser_version", "5.0%20%28Windows%20NT%2010.0%3B%20Win64%3B%20x64%29%20AppleWebKit%2F537.36%20%28KHTML%2C%20like%20Gecko%29%20Chrome%2F109.0.0.0%20Safari%2F537.36"),
                    ("channel", "tiktok_web"),
                    ("cookie_enabled", "false"),
                    ("count", "16"),
                    ("device_id", "7192472737391625774"),
                    ("device_platform", "web_pc"),
                    ("focus_state", "true"),
                    ("from_page", "video"),
                    ("history_len", "2"),
                    ("is_fullscreen", "false"),
                    ("is_page_visible", "true"),
                    ("itemID", itoa::Buffer::new().format(item_id)),
                    ("language", "en"),
                    ("os", "windows"),
                    ("priority_region", ""),
                    ("referer", ""),
                    ("region", "US"),
                    ("screen_height", "864"),
                    ("screen_width", "1536"),
                    ("tz_name", "America%2FLos_Angeles"),
                    ("webcast_language", "en"),
                ],
            )
            .unwrap();

        let text = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
