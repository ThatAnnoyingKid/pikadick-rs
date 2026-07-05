mod ascii_table;
mod encoder_task;
mod loading_reaction;
mod timed_cache;

pub use self::{
    ascii_table::AsciiTable,
    encoder_task::EncoderTask,
    loading_reaction::LoadingReaction,
    timed_cache::{
        TimedCache,
        TimedCacheEntry,
    },
};
use regex::Regex;
use std::{
    path::Path,
    sync::LazyLock,
};
use url::Url;

/// Source: <https://urlregex.com/>
static URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(include_str!("url_regex.txt")).expect("invalid url regex"));

/// Get an iterator over urls in text.
pub fn extract_urls(text: &str) -> impl Iterator<Item = Url> + '_ {
    // Regex doesn't HAVE to be perfect.
    // Ideally, it just needs to be aggressive since parsing it into a url will weed out invalids.
    URL_REGEX
        .find_iter(text)
        .filter_map(|url_match| Url::parse(url_match.as_str()).ok())
}

/// Try to get metadata for a path.
///
/// # Returns
/// Returns `None` if the path does not exist.
pub async fn try_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<Option<std::fs::Metadata>> {
    match tokio::fs::metadata(&path).await {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
