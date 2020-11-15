use crate::{
    types::{
        ApiResponse,
        Article,
    },
    FmlError,
    FmlResult,
};
use bytes::buf::BufExt;
use hyper::header::HeaderValue;
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct Client {
    client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    api_key: String,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);

        Client { client, api_key }
    }

    pub async fn send_req<T: DeserializeOwned>(
        &self,
        mut req: hyper::Request<hyper::Body>,
    ) -> FmlResult<T> {
        req.headers_mut()
            .insert("X-VDM-Api-Key", HeaderValue::from_str(&self.api_key)?);
        let res = self.client.request(req).await?;

        let status = res.status();
        if !status.is_success() {
            return Err(FmlError::InvalidStatus(status));
        }

        let body = hyper::body::aggregate(res.into_body()).await?;
        let res: ApiResponse<T> = serde_json::from_reader(body.reader())?;

        match res {
            ApiResponse::<T>::Ok { data, .. } => Ok(data),
            ApiResponse::<T>::Err { error, .. } => Err(FmlError::Api(error)),
        }
    }

    pub async fn list_random(&self, n: usize) -> FmlResult<Vec<Article>> {
        let url = format!("https://www.fmylife.com/api/v2/article/list?page[number]=1&page[bypage]={}&orderby[RAND()]=ASC", n);
        let req = hyper::Request::get(url).body(hyper::Body::empty())?;
        self.send_req(req).await
    }
}
