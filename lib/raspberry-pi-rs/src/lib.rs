#![cfg(all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux"))]

/// Wrapper code for the c shared library files
#[cfg(feature = "wrapper")]
mod wrapper;
#[cfg(feature = "wrapper")]
pub use self::wrapper::RaspberryPi;
#[cfg(feature = "wrapper")]
use std::os::raw::c_int;

/// Ports of the `bcm_host_*` family of functions.
pub mod bcm_host;

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to load a library
    #[cfg(feature = "wrapper")]
    #[error("failed to load `{name}`")]
    LibraryLoad {
        /// The library name
        name: &'static str,
        #[source]
        error: libloading::Error,
    },

    /// bcm_host is not initialized
    #[cfg(feature = "wrapper")]
    #[error("bcm_host is not initialized")]
    BcmHostNotInitialized,

    /// A board type was unknown
    #[cfg(feature = "wrapper")]
    #[error("the board type `{0}` was unknown")]
    UnknownBoardType(c_int),

    /// `graphics_get_display_size` failed with an error code
    #[cfg(feature = "wrapper")]
    #[error("`graphics_get_display_size` failed with error code `{0}`")]
    GraphicsGetDisplaySize(i32),

    /// A processor id was unknown
    #[cfg(feature = "wrapper")]
    #[error("the processor id `{0}` is unknown")]
    UnknownProcessorId(c_int),

    /// Failed to conver to CString
    #[cfg(feature = "wrapper")]
    #[error(transparent)]
    InteriorNul(#[from] std::ffi::NulError),

    /// A vc_gencmd error
    #[cfg(feature = "wrapper")]
    #[error("a vc gen cmd function failed with error code`{0}`")]
    VcGenCmd(c_int),

    /// A VCos Error
    #[cfg(feature = "wrapper")]
    #[error("a vcos function failed with error code `{0}`")]
    VCos(raspberry_pi_sys::libbcm_host::VCOS_STATUS_T),

    /// Io Error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Parse int error
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// Something was not found
    #[error("not found")]
    NotFound,
}
