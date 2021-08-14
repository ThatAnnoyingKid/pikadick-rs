use reqwest::Body;
use std::{
    borrow::Cow,
    path::Path,
};
use tokio_util::codec::{
    BytesCodec,
    FramedRead,
};

/// An Image
pub enum Image {
    /// A url to an image
    Url(String),

    /// An image file
    File { name: String, body: Body },
}

impl Image {
    /// Make an [`Image`] from a path, opening the file asynchronously.
    pub async fn from_path(path: &Path) -> std::io::Result<Self> {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or(Cow::Borrowed("file.png"))
            .into();
        let file = tokio::fs::File::open(path).await?;
        Self::from_file(name, file)
    }

    /// Make an [`Image`] from a file and a name.
    pub fn from_file(name: String, file: tokio::fs::File) -> std::io::Result<Self> {
        // What a horrible, horrible, horrible interface...
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);
        Ok(Self::File { name, body })
    }
}

impl From<String> for Image {
    fn from(url: String) -> Self {
        Image::Url(url)
    }
}

impl From<&str> for Image {
    fn from(url: &str) -> Self {
        Image::Url(url.into())
    }
}
