/// The error that occurs when a `SearchResult` could not be parsed.
pub type SearchResultError = crate::types::search_result::FromHtmlError;

/// The Error that occurs when a `Post` could not be parsed.
pub type PostError = crate::types::post::FromHtmlError;

/// Crate Error Type
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid URL Error
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    /// IO Error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Invalid Search Result
    #[error("invalid search result")]
    InvalidSearchResult(#[from] SearchResultError),

    /// InvalidPost
    #[error("invalid post")]
    InvalidPost(#[from] PostError),

    /// A tokio task failed to complete
    #[error("failed to join tokio task")]
    TokioJoin(#[from] tokio::task::JoinError),
}
