/// Client type
mod client;
/// Library types
pub mod types;

pub use self::{
    client::Client,
    types::PostPage,
};

/// Error Type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// A Tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// failed to parse a [`PostPage`]
    #[error("invalid post page")]
    InvalidPostPage(#[from] self::types::post_page::FromHtmlError),
}

#[cfg(test)]
mod test {
    use super::*;

    // Only works locally
    #[tokio::test]
    #[ignore]
    async fn download() {
        let urls = [
            "https://vm.tiktok.com/TTPdrksrdc/",
            "https://www.tiktok.com/t/ZTRQsJaw1/",
        ];
        for url in urls {
            let client = Client::new();

            let post = client.get_post(url).await.expect("failed to get post");

            // dbg!(&post);
            dbg!(&post.get_video_download_url());
            dbg!(
                &post
                    .sigi_state
                    .item_module
                    .posts
                    .values()
                    .next()
                    .expect("missing post")
                    .video
            );
        }
    }
}
