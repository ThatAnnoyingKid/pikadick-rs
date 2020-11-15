use crate::{
    types::{
        Post,
        SearchResult,
    },
    RuleResult,
};
use bytes::{
    buf::ext::BufExt,
    Buf,
};
use hyper_tls::HttpsConnector;
use select::document::Document;
use url::Url;

const DEFAULT_USER_AGENT_STR: &str = "rule34-rs";

/// Client
#[derive(Debug)]
pub struct Client {
    client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);

        Client { client }
    }

    /// Gets a uri as a Buf
    pub async fn get_buf(&self, uri: &str) -> RuleResult<impl Buf> {
        let req = hyper::Request::builder()
            .method("GET")
            .uri(uri)
            .header(http::header::USER_AGENT, DEFAULT_USER_AGENT_STR)
            .body(hyper::Body::empty())?;

        let res = self.client.request(req).await?;

        let body = hyper::body::aggregate(res.into_body()).await?;

        Ok(body)
    }

    /// Runs a search. Querys are based on "tags". Tags are seperated by spaces, while words are seperated by underscores. Characters are automatically encoded.
    pub async fn search(&self, query: &str) -> RuleResult<SearchResult> {
        let url = Url::parse_with_params(
            "https://rule34.xxx/index.php?page=post&s=list",
            &[("tags", query)],
        )?;

        let res = self.get_buf(url.as_str()).await?;
        let doc = Document::from_read(res.reader())?;
        let ret = SearchResult::from_doc(&doc)?;

        Ok(ret)
    }

    /// Gets a post by id
    pub async fn get_post(&self, id: u64) -> RuleResult<Post> {
        let url = format!("https://rule34.xxx/index.php?page=post&s=view&id={}", id);

        let res = self.get_buf(&url).await?;
        let doc = Document::from_read(res.reader())?;
        let post = Post::from_doc(&doc)?;

        Ok(post)
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
