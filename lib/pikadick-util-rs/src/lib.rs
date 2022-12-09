#[cfg(feature = "download_to_file")]
pub use nd_util::download_to_file;

#[cfg(feature = "arc_anyhow_error")]
mod arc_anyhow_error;
#[cfg(feature = "arc_anyhow_error")]
pub use self::arc_anyhow_error::ArcAnyhowError;

#[cfg(feature = "async_lock_file")]
mod async_lock_file;
#[cfg(feature = "async_lock_file")]
pub use self::async_lock_file::AsyncLockFile;

#[cfg(feature = "drop_remove_file")]
mod drop_remove_file;
#[cfg(feature = "drop_remove_file")]
pub use self::drop_remove_file::{
    DropRemoveFile,
    DropRemovePath,
};

#[cfg(feature = "download_to_path")]
mod download_to_path;
#[cfg(feature = "download_to_path")]
pub use self::download_to_path::download_to_path;

pub use nd_util::{
    push_extension,
    with_push_extension,
};
