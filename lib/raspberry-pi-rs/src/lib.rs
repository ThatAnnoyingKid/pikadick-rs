#![cfg(all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux"))]

/// Wrapper code for the c shared library files
mod wrapper;

pub use self::wrapper::RaspberryPi;
use std::os::raw::c_int;

/// The error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to load a library
    #[error("failed to load `{name}`")]
    LibraryLoad {
        /// The library name
        name: &'static str,
        #[source]
        error: libloading::Error,
    },

    /// bcm_host is not initialized
    #[error("bcm_host is not initialized")]
    BcmHostNotInitialized,

    /// A board type was unknown
    #[error("the board type `{0}` was unknown")]
    UnknownBoardType(c_int),

    /// `graphics_get_display_size` failed with an error code
    #[error("`graphics_get_display_size` failed with error code `{0}`")]
    GraphicsGetDisplaySize(i32),

    /// A processor id was unknown
    #[error("the processor id `{0}` is unknown")]
    UnknownProcessorId(c_int),
}

/// The board type
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BoardType {
    ModelA,
    ModelB,
    ModelAPlus,
    ModelBPlus,
    Pi2ModelB,
    Alpha,
    Cm,
    Cm2,
    Pi3ModelB,
    Pi0,
    Cm3,
    Custom,
    Pi0W,
    Pi3ModelBPlus,
    Pi3ModelAPlus,
    Fpga,
    Cm3Plus,
    Pi4ModelB,
    Pi400,
    Cm4,
}

/// The id of the processor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessorId {
    Bcm2835,
    Bcm2836,
    Bcm2837,

    /// This is also Bcm2838, which is a deprecated name for this id.
    Bcm2711,
}
