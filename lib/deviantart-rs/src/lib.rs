/// Client
pub mod client;
/// Api Types
pub mod types;

pub use crate::{
    client::{
        Client,
        CookieJar,
    },
    types::{
        Deviation,
        DeviationExtended,
        OEmbed,
        ScrapedWebPageInfo,
        SearchResults,
    },
};

/// Library Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid Url
    #[error(transparent)]
    Url(#[from] url::ParseError),

    /// A tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Missing media token
    #[error("missing media token")]
    MissingMediaToken,

    /// Io Error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Json failed to parse
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Missing the InitialState variable
    #[error("missing initial state")]
    MissingInitialState,

    /// A Cookie Store error
    #[error(transparent)]
    CookieStore(cookie_store::Error),
}
