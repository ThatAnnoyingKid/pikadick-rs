use crate::{
    types::{
        ApiResponse,
        Article,
    },
    FmlResult,
};

/// An FML Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    api_key: String,
}

impl Client {
    /// Make a new Client from an api key
    pub fn new(api_key: String) -> Self {
        Client {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Get a list of random articles.
    pub async fn list_random(&self, n: usize) -> FmlResult<Vec<Article>> {
        let url = format!("https://www.fmylife.com/api/v2/article/list?page[number]=1&page[bypage]={}&orderby[RAND()]=ASC", n);
        let text = self
            .client
            .get(&url)
            .header("X-VDM-Api-Key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let api_response: ApiResponse<_> = serde_json::from_str(&text)?;
        api_response.into()
    }
}
