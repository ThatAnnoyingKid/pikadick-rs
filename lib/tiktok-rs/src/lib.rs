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
            // Old url, deleted?
            // "https://vm.tiktok.com/TTPdrksrdc/",
            "https://www.tiktok.com/t/ZTRQsJaw1/",
        ];
        for url in urls {
            let client = Client::new();

            let post = client.get_post(url).await.expect("failed to get post");
            let item_id = post
                .sigi_state
                .item_module
                .posts
                .keys()
                .next()
                .expect("missing item_id");
            let download_url = post.get_video_download_url().expect("missing download url");

            /*
            let _text = client
                .get_related_item_list(item_id.parse().expect("invalid item id"))
                .await
                .expect("failed to get related items");
            */

            // dbg!(&post);
            dbg!(item_id);
            dbg!(download_url.as_str());
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

            client
                .client
                .get(download_url.as_str())
                .send()
                .await
                .expect("failed to send request")
                .error_for_status()
                .expect("invalid status code");
        }
    }
}
