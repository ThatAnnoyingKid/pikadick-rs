use crate::{
    Error,
    LoginResponse,
    MediaInfo,
    PostPage,
    USER_AGENT_STR,
};
use reqwest_cookie_store::CookieStoreMutex;
use scraper::Html;
use std::sync::Arc;

/// A Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    ///
    /// This probably shouldn't be used by you.
    pub client: reqwest::Client,

    /// The inner cookie store.
    ///
    /// This probably shouldn't be used by you.
    pub cookie_store: Arc<CookieStoreMutex>,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        let cookie_store = Arc::new(CookieStoreMutex::new(Default::default()));
        Self::with_cookie_store(cookie_store)
    }

    /// Make a new [`Client`] from a CookieStore.
    pub fn with_cookie_store(cookie_store: Arc<CookieStoreMutex>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
        );
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_static("https://www.instagram.com/"),
        );

        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT_STR)
            .default_headers(headers)
            .cookie_provider(cookie_store.clone())
            .build()
            .expect("failed to build insta client");

        Client {
            client,
            cookie_store,
        }
    }

    /// Log in
    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse, Error> {
        // TODO: Only run a get on the login page if we are missing a csrf token
        // Get CSRF Cookie
        self.client
            .get("https://www.instagram.com/accounts/login")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let csrf_token = {
            let cookie_store = self.cookie_store.lock().expect("cookie store poisoned");
            cookie_store
                .get("instagram.com", "/", "csrftoken")
                .ok_or(Error::MissingCsrfToken)?
                .value()
                .to_string()
        };

        let response = self
            .client
            .post("https://www.instagram.com/accounts/login/ajax/")
            .header("X-CSRFToken", csrf_token)
            .form(&[("username", username), ("password", password)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response)
    }

    /// Send a GET to a url and return the response.
    ///
    /// This returns an error if the instagram forces the user to log in.
    async fn get_response(&self, url: &str) -> Result<reqwest::Response, Error> {
        let response = self.client.get(url).send().await?.error_for_status()?;

        if response.url().path() == "/accounts/login/" {
            return Err(Error::LoginRequired);
        }

        Ok(response)
    }

    /// Get a post page by url
    pub async fn get_post_page(&self, url: &str) -> Result<PostPage, Error> {
        let text = self.get_response(url).await?.text().await?;
        Ok(tokio::task::spawn_blocking(move || {
            let html = Html::parse_document(&text);
            PostPage::from_html(&html)
        })
        .await??)
    }

    /// Get the media info for a media id
    pub async fn get_media_info(&self, media_id: u64) -> Result<MediaInfo, Error> {
        let url = format!("https://i.instagram.com/api/v1/media/{media_id}/info/");

        let response = self.get_response(url.as_str()).await?;
        Ok(response.json().await?)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
