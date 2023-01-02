/// The Error that occurs when a `HtmlPost` could not be parsed.
pub type HtmlPostError = crate::types::html_post::FromHtmlError;

/// Crate Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid URL Error
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    /// Invalid json
    #[error(transparent)]
    InvalidJson(#[from] serde_json::Error),

    /// Invalid Post
    #[error("invalid html post")]
    InvalidHtmlPost(#[from] HtmlPostError),

    /// A tokio task failed to join
    #[error("failed to join tokio task")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// XML deserialization error
    #[error(transparent)]
    XmlDeserialize(#[from] quick_xml::DeError),

    /// The limit was too large
    #[error("the limit `{0}` is too large")]
    LimitTooLarge(u16),
}
