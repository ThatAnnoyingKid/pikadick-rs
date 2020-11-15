pub type RuleResult<T> = Result<T, RuleError>;

pub type SearchResultError = crate::types::search_result::FromDocError;
pub type PostError = crate::types::post::FromDocError;

#[derive(Debug)]
pub enum RuleError {
    InvalidStatus(hyper::StatusCode),

    Hyper(hyper::Error),
    Http(http::Error),
    Io(std::io::Error),
    InvalidUrl(url::ParseError),

    InvalidSearchResult(SearchResultError),
    InvalidPost(PostError),
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::InvalidStatus(status) => write!(f, "invalid status {}", status),
            RuleError::Hyper(e) => e.fmt(f),
            RuleError::Http(e) => e.fmt(f),
            RuleError::Io(e) => e.fmt(f),
            RuleError::InvalidUrl(e) => e.fmt(f),
            RuleError::InvalidSearchResult(e) => write!(f, "invalid search result: {}", e),
            RuleError::InvalidPost(e) => write!(f, "invalid post: {}", e),
        }
    }
}

impl std::error::Error for RuleError {}

impl From<hyper::Error> for RuleError {
    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}

impl From<http::Error> for RuleError {
    fn from(e: http::Error) -> Self {
        Self::Http(e)
    }
}

impl From<std::io::Error> for RuleError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<url::ParseError> for RuleError {
    fn from(e: url::ParseError) -> Self {
        Self::InvalidUrl(e)
    }
}

impl From<SearchResultError> for RuleError {
    fn from(e: SearchResultError) -> Self {
        Self::InvalidSearchResult(e)
    }
}

impl From<PostError> for RuleError {
    fn from(e: PostError) -> Self {
        Self::InvalidPost(e)
    }
}
