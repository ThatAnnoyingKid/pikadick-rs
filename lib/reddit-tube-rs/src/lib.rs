mod client;
pub mod types;

pub use self::client::Client;
pub use crate::types::{
    GetVideoResponse,
    MainPage,
};
pub use reqwest::StatusCode;

/// Result Type
pub type TubeResult<T> = Result<T, TubeError>;

/// Client Error
#[derive(Debug, thiserror::Error)]
pub enum TubeError {
    /// HTTP Reqwest Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid HTTP Status
    #[error("invalid http status '{0}'")]
    InvalidStatus(StatusCode),

    /// invalid json parse
    #[error("{0}")]
    Json(#[from] serde_json::Error),

    /// invalid main page
    #[error("invalid main page: {0}")]
    InvalidMainPage(#[from] crate::types::main_page::FromHtmlError),

    /// a tokio task failed
    #[error("{0}")]
    TokioJoin(#[from] tokio::task::JoinError),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let video_url = "https://www.reddit.com/r/dankvideos/comments/h8p0py/pp_removal_time/?utm_source=share&utm_medium=web2x";
        let client = Client::new();
        let main_page = client.get_main_page().await.unwrap();
        let vid = client.get_video(&main_page, video_url).await.unwrap();

        dbg!(vid);
    }
}
