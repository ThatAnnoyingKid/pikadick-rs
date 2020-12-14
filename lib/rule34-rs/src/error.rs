pub type RuleResult<T> = Result<T, RuleError>;

pub type SearchResultError = crate::types::search_result::FromDocError;
pub type PostError = crate::types::post::FromDocError;

#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    /// Reqwest HTTP Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    /// Invalid URL Error
    #[error("{0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Invalid Search Result
    #[error("invalid search result: {0}")]
    InvalidSearchResult(#[from] SearchResultError),
    /// InvalidPost
    #[error("invalid post: {0}")]
    InvalidPost(#[from] PostError),
}
