/// The Error that occurs when a `Post` could not be parsed.
pub type PostError = crate::types::post::FromHtmlError;

/// Crate Error Type
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    /// Reqwest HTTP Error
    #[error("reqwest error")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid URL Error
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    /// Invalid json
    #[error("invalid json")]
    InvalidJson(#[from] serde_json::Error),

    /// IO Error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Invalid Post
    #[error("invalid post")]
    InvalidPost(#[from] PostError),

    /// A tokio task failed to complete
    #[error("failed to join tokio task")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// XML deserialization error
    #[error(transparent)]
    XmlDeserialize(#[from] quick_xml::DeError),
}
