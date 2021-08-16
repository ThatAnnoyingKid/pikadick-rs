use crate::{
    Error,
    Image,
    SearchJson,
};
use std::sync::Arc;
use url::Url;

/// The sauce nao client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: Arc<str>,
}

impl Client {
    /// Create a new [`Client`].
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Arc::from(api_key),
        }
    }

    /// Look up an image
    pub async fn search(&self, image: impl Into<Image>) -> Result<SearchJson, Error> {
        let image = image.into();
        let mut url = Url::parse_with_params(
            "https://saucenao.com/search.php?output_type=2",
            &[("api_key", &*self.api_key)],
        )?;

        let mut part = None;
        match image {
            Image::Url(image_url) => {
                url.query_pairs_mut().append_pair("url", &image_url);
            }
            Image::File { name, body } => {
                part = Some(reqwest::multipart::Part::stream(body).file_name(name));
            }
        }

        let mut res = self.client.post(url.as_str());
        if let Some(part) = part {
            let form = reqwest::multipart::Form::new().part("file", part);
            res = res.multipart(form);
        }
        Ok(res.send().await?.error_for_status()?.json().await?)
    }
}
