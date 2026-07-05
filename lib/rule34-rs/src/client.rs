mod note_list_query_builder;
mod post_list_query_builder;
mod tag_list_query_builder;

pub use self::{
    note_list_query_builder::NotesListQueryBuilder,
    post_list_query_builder::PostListQueryBuilder,
    tag_list_query_builder::TagListQueryBuilder,
};
#[cfg(feature = "scrape")]
use crate::HtmlPost;
use crate::{
    AutocompleteResult,
    DeletedImageList,
    Error,
    RATELIMIT_BUDGET,
};
use reqwest::{
    StatusCode,
    header::{
        HeaderMap,
        HeaderValue,
    },
};
#[cfg(feature = "scrape")]
use scraper::Html;
use std::{
    num::NonZeroU64,
    sync::{
        Arc,
        Mutex,
    },
    time::{
        Duration,
        Instant,
    },
};
use url::Url;

// Default Header values
static USER_AGENT_VALUE: HeaderValue = HeaderValue::from_static(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4514.0 Safari/537.36",
);
static REFERER_VALUE: HeaderValue = HeaderValue::from_static("https://rule34.xxx/");
static ACCEPT_LANGUAGE_VALUE: HeaderValue = HeaderValue::from_static("en,en-US;q=0,5");
static ACCEPT_VALUE: HeaderValue = HeaderValue::from_static("*/*");

const ONE_MINUTE: Duration = Duration::from_mins(1);

#[derive(Debug)]
struct AuthState {
    user_id: u64,
    api_key: String,
}

#[derive(Debug)]
struct RatelimitState {
    ratelimit_last_time: Instant,
    budget: u8,
}

#[derive(Debug)]
struct ClientState {
    auth_state: Mutex<Option<AuthState>>,
    ratelimit_state: Mutex<RatelimitState>,
}

/// A Rule34 Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    ///
    /// This probably shouldn't be used by you.
    pub client: reqwest::Client,

    /// The client state.
    state: Arc<ClientState>,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(reqwest::header::USER_AGENT, USER_AGENT_VALUE.clone());
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            ACCEPT_LANGUAGE_VALUE.clone(),
        );
        default_headers.insert(reqwest::header::ACCEPT, ACCEPT_VALUE.clone());
        default_headers.insert(reqwest::header::REFERER, REFERER_VALUE.clone());

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build rule34 client");

        Client {
            client,
            state: Arc::new(ClientState {
                auth_state: std::sync::Mutex::new(None),
                ratelimit_state: Mutex::new(RatelimitState {
                    ratelimit_last_time: Instant::now(),
                    budget: RATELIMIT_BUDGET,
                }),
            }),
        }
    }

    /// Set the authentication.
    pub fn set_auth(&self, user_id: u64, api_key: &str) {
        let mut auth_state = self
            .state
            .auth_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        *auth_state = Some(AuthState {
            user_id,
            api_key: api_key.into(),
        });
    }

    /// Get the auth state.
    fn get_auth(&self) -> std::sync::MutexGuard<'_, Option<AuthState>> {
        self.state
            .auth_state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    async fn ratelimit(&self) {
        loop {
            let sleep_time = {
                let mut state = self
                    .state
                    .ratelimit_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let mut elapsed = state.ratelimit_last_time.elapsed();
                if elapsed > ONE_MINUTE {
                    state.ratelimit_last_time = Instant::now();
                    state.budget = RATELIMIT_BUDGET;
                    elapsed = Duration::ZERO;
                }

                if state.budget > 0 {
                    state.budget -= 1;
                    None
                } else {
                    Some(ONE_MINUTE - elapsed)
                }
            };

            match sleep_time {
                Some(sleep_time) => {
                    tokio::time::sleep(sleep_time).await;
                }
                None => return,
            }
        }
    }

    /// Get the amount ratelimit budget remaining for this time interval.
    pub fn ratelimit_budget_remaining(&self) -> u8 {
        let state = self
            .state
            .ratelimit_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.budget
    }

    async fn ratelimited<FN, FUT, R>(&self, mut func: FN) -> Result<R, Error>
    where
        FN: FnMut() -> FUT,
        FUT: Future<Output = Result<R, Error>>,
    {
        const MAX_TRIES: usize = 3;

        let mut tries = 0;
        loop {
            self.ratelimit().await;

            let future = func();
            let result = future.await;

            match result {
                Ok(value) => return Ok(value),
                Err(Error::Reqwest(error))
                    if error.status() == Some(StatusCode::TOO_MANY_REQUESTS) => {}
                Err(error) => return Err(error),
            }

            tries += 1;
            if tries == MAX_TRIES {
                return Err(Error::Ratelimited);
            }
        }
    }

    /// Send a GET web request to a `url` and get the result as a [`String`].
    async fn get_text(&self, url: &str) -> Result<String, Error> {
        Ok(self
            .client
            .get(url)
            .timeout(Duration::from_secs(90))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    /// Send a GET web request to a `uri` and get the result as [`Html`],
    /// then use the given func to process it.
    #[cfg(feature = "scrape")]
    async fn get_html<F, T>(&self, uri: &str, f: F) -> Result<T, Error>
    where
        F: FnOnce(Html) -> T + Send + 'static,
        T: Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret =
            tokio::task::spawn_blocking(move || f(Html::parse_document(text.as_str()))).await?;
        Ok(ret)
    }

    /// Send a GET web request to a `uri` and get the result as xml, deserializing it to the given type.
    async fn get_xml<T>(&self, uri: &str) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let text = self.get_text(uri).await?;
        let ret = tokio::task::spawn_blocking(move || quick_xml::de::from_str(&text)).await??;
        Ok(ret)
    }

    /// Create a builder to list posts from rule34.
    pub fn list_posts(&self) -> PostListQueryBuilder<'_> {
        PostListQueryBuilder::new(self)
    }

    /// Get a [`HtmlPost`] by `id`.
    #[cfg(feature = "scrape")]
    pub async fn get_html_post(&self, id: NonZeroU64) -> Result<HtmlPost, Error> {
        let url = crate::post_id_to_html_post_url(id);
        let ret = self
            .get_html(url.as_str(), |html| HtmlPost::from_html(&html))
            .await??;

        Ok(ret)
    }

    /// Get a list of deleted images.
    ///
    /// Only include ids over `last_id`. Use `None` for no limit.
    ///
    /// # Warning
    /// Due to current technical limitations,
    /// this function is not very memory efficient depending on `last_id`.
    /// This will require buffering ~30MB into memory.
    /// You should probably limit its use with a semaphore or similar.
    pub async fn list_deleted_images(
        &self,
        last_id: Option<NonZeroU64>,
    ) -> Result<DeletedImageList, Error> {
        let mut url = Url::parse_with_params(
            crate::API_BASE_URL,
            &[
                ("page", "dapi"),
                ("s", "post"),
                ("q", "index"),
                ("deleted", "show"),
            ],
        )?;
        if let Some(last_id) = last_id {
            let mut last_id_buf = itoa::Buffer::new();
            url.query_pairs_mut()
                .append_pair("last_id", last_id_buf.format(last_id.get()));
        }

        {
            let auth = self.get_auth();
            let auth = auth.as_ref().ok_or(Error::MissingAuth)?;

            let mut query_pairs_mut = url.query_pairs_mut();

            query_pairs_mut.append_pair("user_id", itoa::Buffer::new().format(auth.user_id));
            query_pairs_mut.append_pair("api_key", &auth.api_key);
        }

        // Parse on a threadpool since the full returned string is currently around 30 megabytes in size,
        // and we need to run in under a few milliseconds.
        // We need to buffer this all in memory though, since `quick_xml` does not provide a streaming api.
        self.get_xml(url.as_str()).await
    }

    /// Get a builder to list tags.
    pub fn list_tags(&self) -> TagListQueryBuilder<'_> {
        TagListQueryBuilder::new(self)
    }

    /// Get a builder to list notes.
    ///
    /// This is undocumented.
    pub fn list_notes(&self) -> NotesListQueryBuilder<'_> {
        NotesListQueryBuilder::new(self)
    }

    /// Autocomplete a tag.
    pub async fn autocomplete(&self, query: &str) -> Result<Vec<AutocompleteResult>, Error> {
        let url =
            Url::parse_with_params("https://api.rule34.xxx/autocomplete.php", &[("q", query)])?;
        let response = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?;
        let data = response.json().await?;
        Ok(data)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
