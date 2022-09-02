mod ascii_table;
mod encoder_task;
mod loading_reaction;
mod request_map;
mod timed_cache;
pub mod twilight_loading_reaction;

pub use self::{
    ascii_table::AsciiTable,
    encoder_task::EncoderTask,
    loading_reaction::LoadingReaction,
    request_map::RequestMap,
    timed_cache::{
        TimedCache,
        TimedCacheEntry,
    },
    twilight_loading_reaction::TwilightLoadingReaction,
};
pub use pikadick_util::{
    download_to_file,
    with_push_extension,
    ArcAnyhowError,
    AsyncLockFile,
    DropRemoveFile,
    DropRemovePath,
};

/// Check if a host is a reddit host
pub fn is_reddit_host(url_host: &url::Host<&str>) -> bool {
    matches!(url_host, url::Host::Domain("www.reddit.com" | "reddit.com"))
}

/// Check if a url host is a tiktok host
pub fn is_tiktok_host(url_host: &url::Host<&str>) -> bool {
    matches!(
        url_host,
        url::Host::Domain("vm.tiktok.com" | "tiktok.com" | "www.tiktok.com")
    )
}

/// Get the file extension from a url
pub fn get_extension_from_url(url: &url::Url) -> Option<&str> {
    Some(url.path_segments()?.rev().next()?.rsplit_once('.')?.1)
}
