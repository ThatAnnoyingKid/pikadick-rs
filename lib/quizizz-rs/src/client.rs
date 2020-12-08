use crate::{
    obfs::decode::decode_str,
    types::{
        CheckRoomJsonRequest,
        CheckRoomObfuscatedJsonResponse,
        GenericResponse,
    },
    QError,
    QResult,
};
use bytes::Buf;
use hyper::{
    header::{
        CONTENT_LENGTH,
        CONTENT_TYPE,
    },
    Method,
    Request,
};
use hyper_tls::HttpsConnector;

const CHECK_ROOM_URI: &str = "https://game.quizizz.com/play-api/v3/checkRoom";

#[derive(Debug)]
pub struct Client {
    client: hyper::Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);

        Self { client }
    }

    pub async fn check_room_v3(&self, room_code: &'_ str) -> QResult<GenericResponse> {
        let payload = serde_json::to_vec(&CheckRoomJsonRequest { room_code })?;

        let req = Request::builder()
            .method(Method::POST)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, payload.len())
            .uri(CHECK_ROOM_URI)
            .body(hyper::Body::from(payload))?;

        let res = self.client.request(req).await?;

        let status = res.status();
        if !status.is_success() {
            return Err(QError::InvalidStatus(status));
        }

        let body = hyper::body::aggregate(res.into_body()).await?;
        let res: CheckRoomObfuscatedJsonResponse = serde_json::from_slice(body.bytes())?;

        let ret = decode_str(&res.odata).ok_or(QError::Decode)?;
        let ret: GenericResponse = serde_json::from_str(&ret)?;

        Ok(ret)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
