mod arc_anyhow_error;
mod ascii_table;
mod encoder_task;
mod loading_reaction;
mod request_map;
mod timed_cache;

pub use self::{
    arc_anyhow_error::ArcAnyhowError,
    ascii_table::AsciiTable,
    encoder_task::EncoderTask,
    loading_reaction::LoadingReaction,
    request_map::RequestMap,
    timed_cache::{
        TimedCache,
        TimedCacheEntry,
    },
};
pub use pikadick_util::download_to_file;
use std::{
    ffi::{
        OsStr,
        OsString,
    },
    path::PathBuf,
};

/// Push an extension to a [`PathBuf`].
pub fn push_extension<S: AsRef<OsStr>>(path: &mut PathBuf, extension: S) {
    let extension = extension.as_ref();

    // Bail out early if there is no extension, simply setting one.
    if path.extension().is_none() {
        path.set_extension(extension);
        return;
    }

    // Take the path memory, make it a string, push the extension, and restore the argument path.
    //
    // Ideally, I woudln't take ownership of the original string,
    // but there is no API to push arbitrary bytes to a [`PathBuf`].
    // Similarly, there is no api to access the underlying [`OsString`] of a [`PathBuf`].
    let mut path_string = OsString::from(std::mem::take(path));
    path_string.reserve(extension.len() + 1);
    path_string.push(".");
    path_string.push(extension);
    std::mem::swap(path, &mut path_string.into());
}
