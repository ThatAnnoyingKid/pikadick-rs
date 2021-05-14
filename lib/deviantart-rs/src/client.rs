use crate::{
    Deviation,
    Error,
    OEmbed,
    ScrapedWebPageInfo,
    SearchResults,
};
use regex::Regex;
use tokio::io::{
    AsyncWrite,
    AsyncWriteExt,
};
use url::Url;

/// A DeviantArt Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client. You probably shouldn't touch this.
    pub client: reqwest::Client,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::builder()
                .cookie_store(true)
                .build()
                .expect("failed to build deviantart client"),
        }
    }

    /// Sign in to get access to more results from apis
    pub async fn signin(&self, username: &str, password: &str) -> Result<(), Error> {
        let scraped_webpage = self.scrape_webpage("https://www.deviantart.com").await?;
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

    /// Search for deviations
    pub async fn search(&self, query: &str) -> Result<SearchResults, Error> {
        let url = Url::parse_with_params(
            "https://www.deviantart.com/_napi/da-browse/api/networkbar/search/deviations",
            &[("q", query), ("page", "1")],
        )?;
        let results = self
            .client
            .get(url.as_str())
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
