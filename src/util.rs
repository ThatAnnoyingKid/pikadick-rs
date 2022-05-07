mod arc_anyhow_error;
mod ascii_table;
mod drop_remove_file;
mod encoder_task;
mod loading_reaction;
mod request_map;
mod timed_cache;

pub use self::{
    arc_anyhow_error::ArcAnyhowError,
    ascii_table::AsciiTable,
    drop_remove_file::{
        DropRemoveFile,
        DropRemovePath,
    },
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
    path::{
        Path,
        PathBuf,
    },
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

/// Push an extension to a [`Path`], returning a new [`PathBuf`].
pub fn with_push_extension<P, S>(path: P, extension: S) -> PathBuf
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let path = path.as_ref();
    let extension = extension.as_ref();

    // Bail out early if there is no extension, simply setting one.
    if path.extension().is_none() {
        return path.with_extension(extension);
    }

    // Change the path into an OsString so we can push arbitrary bytes to it,
    // then change it into a PathBuf so we can return it.
    let mut path_string = OsString::from(path);
    path_string.reserve(extension.len() + 1);
    path_string.push(".");
    path_string.push(extension);
    PathBuf::from(path_string)
}
