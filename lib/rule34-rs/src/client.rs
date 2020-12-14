use crate::{
    types::{
        Post,
        SearchResult,
    },
    RuleError,
    RuleResult,
};
use select::document::Document;
use std::io::Write;
use url::Url;

const DEFAULT_USER_AGENT_STR: &str = "rule34-rs";

/// Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }

    /// Gets a uri as a String
    pub async fn get_text(&self, uri: &str) -> RuleResult<String> {
        let res = self
            .client
            .get(uri)
            .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT_STR)
            .send()
            .await?;

        let text = res.text().await?;

        Ok(text)
    }

    /// Gets a uri as a doc
    pub async fn get_doc(&self, uri: &str) -> RuleResult<Document> {
        Ok(Document::from(self.get_text(uri).await?.as_str()))
    }

    /// Runs a search. Querys are based on "tags". Tags are seperated by spaces, while words are seperated by underscores. Characters are automatically encoded.
    pub async fn search(&self, query: &str) -> RuleResult<SearchResult> {
        let url = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=list",
            &[("tags", query)],
        )?;

        let doc = self.get_doc(url.as_str()).await?;
        let ret = SearchResult::from_doc(&doc)?;

        Ok(ret)
    }

    /// Gets a post by id
    pub async fn get_post(&self, id: u64) -> RuleResult<Post> {
        let url = format!("https://rule34.xxx/index.php?page=post&s=view&id={}", id);

        let doc = self.get_doc(&url).await?;
        let post = Post::from_doc(&doc)?;

        Ok(post)
    }

    /// Get a url and copy it to the given writer
    pub async fn copy_res_to<T: Write>(&self, url: &Url, mut writer: T) -> RuleResult<()> {
        let mut res = self.client.get(url.as_str()).send().await?;
        let status = res.status();
        if !status.is_success() {
            return Err(RuleError::InvalidStatus(status));
        }

        while let Some(chunk) = res.chunk().await? {
            writer.write_all(&chunk)?;
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
mod test {
    use super::*;

    #[tokio::test]
    async fn search() {
        let client = Client::new();
        let res = client.search("rust").await.unwrap();
        dbg!(&res);
        assert!(!res.entries.is_empty());
    }

    async fn get_top_post(query: &str) -> Post {
        let client = Client::new();
        let res = client.search(query).await.unwrap();
        assert!(!res.entries.is_empty());

        let last = res.entries.last().as_ref().unwrap().as_ref().unwrap();
        client.get_post(last.id).await.unwrap()
    }

    #[tokio::test]
    async fn it_works_rust() {
        let post = get_top_post("rust").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_fbi() {
        let post = get_top_post("fbi").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_gif() {
        let post = get_top_post("gif").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_corna() {
        let post = get_top_post("corna").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_sledge() {
        let post = get_top_post("sledge").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_deep() {
        let post = get_top_post("deep").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_roadhog() {
        let post = get_top_post("roadhog").await;
        dbg!(&post);
    }

    #[tokio::test]
    async fn it_works_deep_space_waifu() {
        let post = get_top_post("deep_space_waifu").await;
        dbg!(&post);
    }
}
