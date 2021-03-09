/// The crate Result Type
///
pub type RuleResult<T> = Result<T, RuleError>;

/// The error that occurs when a SearchResult could not be parsed
///
pub type SearchResultError = crate::types::search_result::FromDocError;

/// The Error that occurs when a post could not be parsed
///
pub type PostError = crate::types::post::FromDocError;

/// Crate Error Type
///
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    /// Reqwest HTTP Error
    ///
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid URL Error
    #[error("{0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Invalid HTTP Status Code
    ///
    #[error("invalid status {0}")]
    InvalidStatus(reqwest::StatusCode),

    /// IO Error
    ///
    #[error("{0}")]
    Io(#[from] std::io::Error),

    /// Invalid Search Result
    ///
    #[error("invalid search result: {0}")]
    InvalidSearchResult(#[from] SearchResultError),

    /// InvalidPost
    ///
    #[error("invalid post: {0}")]
    InvalidPost(#[from] PostError),

    /// A tokio task panicked
    ///
    #[error("{0}")]
    TokioJoin(#[from] tokio::task::JoinError),
}
