mod cookie_jar;

pub use self::cookie_jar::CookieJar;
use crate::{
    Deviation,
    Error,
    OEmbed,
    ScrapedStashInfo,
    ScrapedWebPageInfo,
    SearchResults,
};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::{
    HeaderMap,
    HeaderValue,
};
use std::sync::Arc;
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36";

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
        Self::new_with_user_agent(USER_AGENT_STR)
    }

    /// Make a new [`Client`] with the given user agent.
    pub fn new_with_user_agent(user_agent: &str) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en,en-US;q=0,5"),
        );
        default_headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("*/*"));
        default_headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static("https://www.deviantart.com/"),
        );

        let cookie_store = Arc::new(CookieJar::new());
        let client = reqwest::Client::builder()
            .cookie_provider(cookie_store.clone())
            .user_agent(user_agent)
            .default_headers(default_headers)
            .build()
            .expect("failed to build deviantart client");

        Client {
            client,
            cookie_store,
        }
    }

    /// Sign in to get access to more results from apis.
    ///
    /// This will also clean the cookie jar.
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

        // TODO: Verify login
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
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"window\.__INITIAL_STATE__ = JSON\.parse\("(.*)"\);"#)
                .expect("invalid `scrape_deviation` regex")
        });

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

    /// Scrape a sta.sh link for info
    pub async fn scrape_stash_info(&self, url: &str) -> Result<ScrapedStashInfo, Error> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"deviantART.pageData=(.*);"#).expect("invalid `scrape_stash_info` regex")
        });

        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let scraped_stash = tokio::task::spawn_blocking(move || {
            let capture = REGEX
                .captures(&text)
                .and_then(|captures| captures.get(1))
                .ok_or(Error::MissingPageData)?;
            let scraped_stash: ScrapedStashInfo = serde_json::from_str(capture.as_str())?;

            Result::<_, Error>::Ok(scraped_stash)
        })
        .await??;

        Ok(scraped_stash)
    }

    /// Download a [`Deviation`].
    ///
    /// Only works with images. It will attempt to get the highest quality image it can.
    ///
    /// This is discouraged as you really need all the data from a scraped webpage to give a good try at high-quality downloads.
    pub async fn download_deviation(
        &self,
        deviation: &Deviation,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<(), Error> {
        let url = deviation
            .get_download_url()
            .or_else(|| deviation.get_fullview_url())
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
        let results = client.search("sun", 1).await.expect("failed to search");
        // dbg!(&results);
        let _first = &results.deviations[0];
        // dbg!(first);

        // This function is discouraged and will likely be replaced in the future.
        // Since it fails CI spuriously a lot, we will not test it.
        /*
        let image = File::create("test.jpg").await.expect("failed to save file");
        client
            .download_deviation(first, image)
            .await
            .expect("failed to download deviation");
        */
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
        let res = client.search("furry", 1).await.expect("test");

        dbg!(res.deviations.len());
    }
}
