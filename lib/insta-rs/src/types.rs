/// The [`PostPage`] type
mod post_page;

pub use self::post_page::PostPage;

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
