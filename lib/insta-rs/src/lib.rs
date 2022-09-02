/// The client
mod client;
/// API Types
pub mod types;

pub use self::{
    client::Client,
    types::{
        LoginResponse,
        MediaInfo,
        MediaType,
        PostPage,
    },
};
pub use cookie_store::CookieStore;
pub use reqwest_cookie_store::CookieStoreMutex;

const USER_AGENT_STR: &str = "Instagram 123.0.0.21.114 (iPhone; CPU iPhone OS 11_4 like Mac OS X; en_US; en-US; scale=2.00; 750x1334) AppleWebKit/605.1.15";

/// Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Instagram is forcing a log-in
    #[error("login required")]
    LoginRequired,

    /// Missing a csrf token
    #[error("missing csrf token")]
    MissingCsrfToken,

    /// Invalid Post Page
    #[error("invalid post page")]
    InvalidPostPage(#[from] crate::types::post_page::FromHtmlError),

    /// Json
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Tokio join error
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::OnceCell;

    #[derive(Debug, serde::Deserialize)]
    struct TestConfig {
        username: String,
        password: String,
    }

    impl TestConfig {
        fn new() -> Self {
            let data = std::fs::read_to_string("test-config.json")
                .expect("failed to load `test-config.json`");
            serde_json::from_str(&data).expect("failed to parse `test-config.json`")
        }
    }

    async fn get_client() -> &'static Client {
        static CLIENT: OnceCell<Client> = OnceCell::const_new();

        CLIENT
            .get_or_init(|| async move {
                tokio::task::spawn_blocking(|| {
                    use std::{
                        fs::File,
                        io::BufReader,
                    };
                    use tokio::runtime::Handle;

                    let session_file_path = "session.json";

                    match File::open(session_file_path).map(BufReader::new) {
                        Ok(file) => {
                            let cookie_store =
                                CookieStore::load_json(file).expect("failed to load session file");
                            Client::with_cookie_store(Arc::new(CookieStoreMutex::new(cookie_store)))
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            let test_config = TestConfig::new();

                            let client = Client::new();
                            Handle::current().block_on(async {
                                let login_response = client
                                    .login(&test_config.username, &test_config.password)
                                    .await
                                    .expect("failed to log in");

                                assert!(login_response.authenticated);
                            });

                            let mut session_file = File::create(session_file_path)
                                .expect("failed to open session file");

                            client
                                .cookie_store
                                .lock()
                                .expect("cookie jar poisoned")
                                .save_json(&mut session_file)
                                .expect("failed to save to session file");

                            client
                        }
                        Err(e) => {
                            panic!("failed to open session file: {}", e);
                        }
                    }
                })
                .await
                .expect("task failed to join")
            })
            .await
    }

    /// Fails on CI since other people hit the rate limit.
    #[ignore]
    #[tokio::test]
    async fn get_post() {
        let posts = [
            "https://www.instagram.com/p/CIlZpXKFfNt/",
            "https://www.instagram.com/p/Ch4J91UsYvZ/",
            "https://www.instagram.com/p/ChzrLrjsAFK/",
        ];
        let client = get_client().await;

        for post in posts {
            let post_page = client
                .get_post_page(post)
                .await
                .expect("failed to get post page");

            dbg!(&post_page);

            let media_info = client
                .get_media_info(post_page.media_id)
                .await
                .expect("failed to get media info");
            let media_info_item = media_info.items.first().expect("missing item");

            dbg!(&media_info);

            match media_info_item.media_type {
                MediaType::Photo => {
                    let image_versions2 = media_info_item
                        .image_versions2
                        .as_ref()
                        .expect("missing image version");
                    let best = image_versions2
                        .get_best()
                        .expect("failed to get best image");
                    dbg!(&best);
                }
                MediaType::Video => {
                    let video_version = media_info_item
                        .get_best_video_version()
                        .expect("failed to get the best video version");
                    dbg!(&video_version);
                }
                MediaType::Carousel => {
                    let carousel_media = media_info_item
                        .carousel_media
                        .as_ref()
                        .expect("missing carousel");

                    for media in carousel_media {
                        match media.media_type {
                            MediaType::Photo => {
                                let image_versions2 = media
                                    .image_versions2
                                    .as_ref()
                                    .expect("missing image version");
                                let best = image_versions2
                                    .get_best()
                                    .expect("failed to get best image");
                                dbg!(&best);
                            }
                            MediaType::Video => {
                                let video_version = media
                                    .get_best_video_version()
                                    .expect("failed to get the best video version");
                                dbg!(&video_version);
                            }
                            MediaType::Carousel => todo!("nested carousel"),
                        }
                    }
                }
            }
        }
    }
    
    #[ignore]
    #[tokio::test]
    async fn collections_work() {
         let client = get_client().await;
         
         let collections = client.list_collections().await.expect("failed to list collections");
         dbg!(collections);
    }
}
