mod client;
pub mod types;

pub use crate::{
    client::Client,
    types::{
        GetVideoResponse,
        MainPage,
    },
};

/// Client Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP Reqwest Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// invalid main page
    #[error("invalid main page")]
    InvalidMainPage(#[from] crate::types::main_page::FromHtmlError),

    /// a tokio task failed
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn it_works() {
        let video_url = "https://www.reddit.com/r/dankvideos/comments/h8p0py/pp_removal_time/?utm_source=share&utm_medium=web2x";
        let client = Client::new();
        let main_page = client
            .get_main_page()
            .await
            .expect("failed to get main page");
        let vid = client
            .get_video(&main_page, video_url)
            .await
            .expect("failed to get video");

        dbg!(vid);
    }
}
