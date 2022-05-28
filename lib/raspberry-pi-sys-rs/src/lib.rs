#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]

#[cfg(all(target_arch = "arm", target_os = "linux"))]
mod arm_bindings;
#[cfg(all(target_arch = "arm", target_os = "linux"))]
pub use self::arm_bindings::*;

#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
mod arm64_bindings;
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub use self::arm64_bindings::*;
