mod client;
mod types;

pub use self::{
    client::Client,
    types::{
        AdditionalDataLoaded,
        LoginResponse,
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

    /// Missing additionalDataLoaded
    #[error("missing `additionalDataLoaded` field")]
    MissingAdditionalDataLoaded,

    /// Json
    #[error(transparent)]
    Json(#[from] serde_json::Error),
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
        let client = get_client().await;

        let post_page = client
            .get_post("https://www.instagram.com/p/CIlZpXKFfNt/")
            .await
            .expect("failed to get post page");

        let video_version = post_page
            .items
            .first()
            .expect("missing post item")
            .get_best_video_version()
            .expect("failed to get the best video version");

        dbg!(video_version);
    }
}
