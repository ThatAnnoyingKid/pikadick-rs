use crate::{
    types::ImageList,
    ImageUri,
    NekosError,
    NekosResult,
};
use bytes::{
    buf::BufExt,
    Buf,
};
use hyper_tls::HttpsConnector;
use std::{
    convert::TryFrom,
    fmt::Write,
};

#[derive(Debug)]
pub struct Client {
    client: hyper::Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl Client {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Client { client }
    }

    async fn get_body(&self, uri: hyper::Uri) -> NekosResult<impl Buf> {
        let res = self.client.get(uri).await?;
        let status = res.status();

        if !status.is_success() {
            return Err(NekosError::InvalidStatus(status));
        }

        let body = res.into_body();
        let buf = hyper::body::aggregate(body).await?;

        Ok(buf)
    }

    /// Get a random list of catgirls.
    /// count is a num from 0 < count <= 100 and is the number of returned images.
    /// nsfw is whether the images should be nsfw. If not specified, both are returned.
    pub async fn get_random(&self, nsfw: Option<bool>, count: u8) -> NekosResult<ImageList> {
        let mut url = format!(
            "https://nekos.moe/api/v1/random/image?count={}",
            count.min(100)
        );

        if let Some(nsfw) = nsfw {
            write!(&mut url, "&nsfw={}", nsfw).unwrap();
        }

        let uri = hyper::Uri::try_from(url)?;
        let body = self.get_body(uri).await?;
        let json = serde_json::from_reader(body.reader())?;

        Ok(json)
    }

    pub async fn get_image(&self, uri: ImageUri) -> NekosResult<bytes::Bytes> {
        Ok(self.get_body(uri.0).await?.to_bytes())
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

    #[test]
    pub fn common() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let client = Client::new();
        let image_list = rt.block_on(client.get_random(Some(false), 10)).unwrap();

        assert_eq!(image_list.images.len(), 10);

        let image_uri = image_list.images[0].uri().unwrap();
        let _image = rt.block_on(client.get_image(image_uri)).unwrap();
    }
}
