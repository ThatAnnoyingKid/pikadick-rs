/// The [`PostPage`] type
mod post_page;

/// The [`AdditionalDataLoaded`] type
mod additional_data_loaded;

pub use self::{
    additional_data_loaded::{
        AdditionalDataLoaded,
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
