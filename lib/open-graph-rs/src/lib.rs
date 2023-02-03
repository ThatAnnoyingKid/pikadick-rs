#![allow(clippy::uninlined_format_args)]

//! <https://ogp.me/>

/// [`OpenGraphObject`]
pub mod open_graph_object;

/// A generic open graph client
#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use self::client::Client;
pub use self::open_graph_object::OpenGraphObject;
pub use scraper::Html;
