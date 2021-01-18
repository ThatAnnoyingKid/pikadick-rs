use crate::{
    types::{
        ApiResponse,
        Article,
    },
    Error,
    FmlResult,
};

/// An FML Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: String,
}

impl Client {
    /// Make a new Client from an api_key
    pub fn new(api_key: String) -> Self {
        Client {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Get a list of random articles
    pub async fn list_random(&self, n: usize) -> FmlResult<Vec<Article>> {
        let url = format!("https://www.fmylife.com/api/v2/article/list?page[number]=1&page[bypage]={}&orderby[RAND()]=ASC", n);
        let res = self
            .client
            .get(&url)
            .header("X-VDM-Api-Key", &self.api_key)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            return Err(Error::InvalidStatus(status));
        }

        let text = res.text().await?;
        let res = serde_json::from_str(&text)?;

        match res {
            ApiResponse::Ok { data, .. } => Ok(data),
            ApiResponse::Err { error, .. } => Err(Error::Api(error)),
        }
    }
}
