use crate::{
    Deviation,
    Error,
    OEmbed,
    ScrapedWebPageInfo,
    SearchResults,
};
use bytes::Bytes;
use cookie_store::CookieStore;
use regex::Regex;
use reqwest::header::{
    HeaderMap,
    HeaderValue,
};
use std::{
    fmt::Write,
    sync::{
        Arc,
        RwLock,
    },
};
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

/// A Cookie Jar
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CookieJar(RwLock<cookie_store::CookieStore>);

impl CookieJar {
    /// Make a new Cookie Jar.
    pub fn new() -> Self {
        Self(RwLock::new(Default::default()))
    }

    /// Clean the jar of expired cookies
    pub fn clean(&self) {
        let mut cookie_store = self.0.write().expect("cookie jar poisoned");

        let to_remove: Vec<_> = cookie_store
            .iter_any()
            .filter(|cookie| cookie.is_expired())
            .map(|cookie| {
                let domain = cookie
                    .domain()
                    .map(ToString::to_string)
                    .unwrap_or_else(String::new);
                let name = cookie.name().to_string();

                let path = cookie
                    .path()
                    .map(ToString::to_string)
                    .unwrap_or_else(String::new);

                (domain, name, path)
            })
            .collect();

        for (domain, name, path) in to_remove {
            cookie_store.remove(&domain, &name, &path);
        }
    }

    /// Save the cookie jar as json
    pub fn save_json<W>(&self, mut writer: W) -> Result<(), Error>
    where
        W: std::io::Write,
    {
        let cookie_store = self.0.read().expect("cookie jar poisoned");
        cookie_store
            .save_json(&mut writer)
            .map_err(Error::CookieStore)?;
        Ok(())
    }

    /// Load cookies from a json cookie file
    pub fn load_json<R>(&self, mut reader: R) -> Result<(), Error>
    where
        R: std::io::BufRead,
    {
        let mut cookie_store = self.0.write().expect("cookie jar poisoned");
        *cookie_store = CookieStore::load_json(&mut reader).map_err(Error::CookieStore)?;
        Ok(())
    }
}

impl reqwest::cookie::CookieStore for CookieJar {
    fn set_cookies(&self, headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        use cookie::Cookie;

        let iter = headers.filter_map(|val| {
            let val = val.to_str().ok()?;
            let cookie = Cookie::parse(val).ok()?;
            Some(cookie.into_owned())
        });

        self.0
            .write()
            .expect("cookie jar poisoned")
            .store_response_cookies(iter, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let mut val = String::new();
        let cookie_jar = self.0.read().expect("cookie jar poisoned");

        for cookie in cookie_jar.get_request_cookies(url) {
            let name = cookie.name();
            let value = cookie.value();

            val.reserve(name.len() + value.len() + 1 + 1);
            write!(&mut val, "{}={}; ", name, value).ok()?;
        }
        val.pop(); // Remove ' '
        val.pop(); // Remove ';'

        if val.is_empty() {
            None
        } else {
            HeaderValue::from_maybe_shared(Bytes::from(val)).ok()
        }
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// A DeviantArt Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client. You probably shouldn't touch this.
    pub client: reqwest::Client,
    /// The cookie store
    pub cookie_store: Arc<CookieJar>,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT_ENCODING,
            HeaderValue::from_static("identity"),
        );

        let cookie_store = Arc::new(CookieJar::new());

        Client {
            client: reqwest::Client::builder()
                .cookie_provider(cookie_store.clone())
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4508.0 Safari/537.36")
                .default_headers(headers)
                .build()
                .expect("failed to build deviantart client"),
            cookie_store,
        }
    }

    /// Sign in to get access to more results from apis. This will also clean the cookie jar.
    pub async fn signin(&self, username: &str, password: &str) -> Result<(), Error> {
        self.cookie_store.clean();

        let scraped_webpage = self
            .scrape_webpage("https://www.deviantart.com/users/login")
            .await?;
        let res = self
            .client
            .post("https://www.deviantart.com/_sisu/do/signin")
            .form(&[
                ("referer", "https://www.deviantart.com/"),
                ("csrf_token", &scraped_webpage.config.csrf_token),
                ("username", username),
                ("password", password),
                ("challenge", "0"),
                ("remember", "on"),
            ])
            .send()
            .await?
            .error_for_status()?;

        let _text = res.text().await?;

        Ok(())
    }

    /// Run a GET request on the home page and check if the user is logged in
    pub async fn is_logged_in_online(&self) -> Result<bool, Error> {
        Ok(self
            .scrape_webpage("https://www.deviantart.com/")
            .await?
            .public_session
            .is_logged_in)
    }

    /// Search for deviations with the given query.
    ///
    /// Page numbering starts at 1.
    pub async fn search(&self, query: &str, page: u64) -> Result<SearchResults, Error> {
        let mut page_buffer = itoa::Buffer::new();
        let url = Url::parse_with_params(
            "https://www.deviantart.com/_napi/da-browse/api/networkbar/search/deviations",
            &[("q", query), ("page", page_buffer.format(page))],
        )?;
        let results = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(results)
    }

    /// OEmbed API
    pub async fn get_oembed(&self, url: &str) -> Result<OEmbed, Error> {
        let url = Url::parse_with_params("https://backend.deviantart.com/oembed", &[("url", url)])?;
        let res = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    /// Scrape a webpage for info
    pub async fn scrape_webpage(&self, url: &str) -> Result<ScrapedWebPageInfo, Error> {
        lazy_static::lazy_static! {
            static ref REGEX: Regex = Regex::new(r#"window\.__INITIAL_STATE__ = JSON\.parse\("(.*)"\);"#).expect("invalid `scrape_deviation` regex");
        }

        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let scraped_webpage = tokio::task::spawn_blocking(move || {
            let capture = REGEX
                .captures(&text)
                .and_then(|captures| captures.get(1))
                .ok_or(Error::MissingInitialState)?;
            let capture = capture.as_str().replace("\\\"", "\"").replace("\\\\", "\\");
            let scraped_webpage: ScrapedWebPageInfo = serde_json::from_str(&capture)?;

            Result::<_, Error>::Ok(scraped_webpage)
        })
        .await??;

        Ok(scraped_webpage)
    }

    /// Download a [`Deviation`].
    ///
    /// Only works with images. It will attempt to get the highest quality image it can.
    pub async fn download_deviation(
        &self,
        deviation: &Deviation,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<(), Error> {
        let url = deviation
            .get_download_url()
            .or_else(|| deviation.get_media_url())
            .ok_or(Error::MissingMediaToken)?;

        let mut res = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?;

        while let Some(chunk) = res.chunk().await? {
            writer.write_all(&chunk).await?;
        }

        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Config {
        username: String,
        password: String,
    }

    impl Config {
        fn from_path(path: &str) -> Config {
            let file = std::fs::read(path).expect("failed to read config");
            serde_json::from_reader(file.as_slice()).expect("failed to parse config")
        }
    }

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let results = client.search("sun").await.expect("failed to search");
        // dbg!(&results);
        let first = &results.deviations[0];
        // dbg!(first);
        let image = tokio::fs::File::create("test.jpg").await.unwrap();
        client.download_deviation(first, image).await.unwrap();
    }

    #[tokio::test]
    async fn scrape_deviation() {
        let client = Client::new();
        let _scraped_webpage = client
            .scrape_webpage("https://www.deviantart.com/zilla774/art/chaos-gerbil-RAWR-119577071")
            .await
            .expect("failed to scrape webpage");
    }

    #[tokio::test]
    async fn scrape_webpage_literature() {
        let client = Client::new();
        let scraped_webpage = client
            .scrape_webpage("https://www.deviantart.com/tohokari-steel/art/A-Fictorian-Tale-Chapter-11-879180914")
            .await
            .expect("failed to scrape webpage");
        let current_deviation = scraped_webpage
            .get_current_deviation()
            .expect("missing current deviation");
        let text_content = current_deviation
            .text_content
            .as_ref()
            .expect("missing text content");
        let _markup = text_content
            .html
            .get_markup()
            .expect("missing markup")
            .expect("failed to parse markup");
        // dbg!(&markup);
    }

    #[tokio::test]
    #[ignore]
    async fn signin() {
        let config: Config = Config::from_path("config.json");

        let client = Client::new();
        client
            .signin(&config.username, &config.password)
            .await
            .expect("failed to sign in");
        let res = client.search("furry").await.expect("test");

        dbg!(res.deviations.len());
    }
}
