use std::num::NonZeroU64;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PostJson {
    /// The id of this post.
    pub id: NonZeroU64,

    /// The username of the poster.
    ///
    /// This appears to always be lowercase.
    pub owner: String,
}

pub type PostListJson = Vec<PostJson>;
