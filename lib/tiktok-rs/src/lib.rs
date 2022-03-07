/// Client type
mod client;
/// Library types
pub mod types;

pub use self::{
    client::Client,
    types::PostPage,
};
pub use open_graph::{
    self,
    Html,
    OpenGraphObject,
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

    /// Failed to parse an [`OpenGraphObject`].
    #[error("invalid ogp object")]
    InvalidOpenGraphObject(#[from] open_graph::open_graph_object::FromHtmlError),

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
        let url = "https://vm.tiktok.com/TTPdrksrdc/";
        let client = Client::new();

        let post = client.get_post(url).await.expect("failed to get post");

        dbg!(&post);
        dbg!(&post.get_video_download_url());
    }
}
