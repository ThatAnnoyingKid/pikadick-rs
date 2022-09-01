/// The [`MediaInfo`] type
pub mod media_info;
/// The [`PostPage`] type
pub mod post_page;

pub use self::{
    media_info::{
        MediaInfo,
        MediaType,
    },
    post_page::PostPage,
};

/// The response for a login
#[derive(Debug, serde::Deserialize)]
pub struct LoginResponse {
    /// Whether the user successfully logged in
    pub authenticated: bool,

    /// ?
    pub status: String,

    /// ?
    pub user: bool,
}
