use super::Md5Digest;
use std::num::NonZeroU64;

/// A list of deleted images
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct DeletedImageList {
    /// A list of deleted posts
    #[serde(alias = "post", default)]
    pub posts: Vec<Post>,
}

/// A deleted post
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Post {
    /// The deleted post id
    #[serde(alias = "@deleted")]
    pub deleted: NonZeroU64,

    /// The md5 hash of the deleted post
    #[serde(alias = "@md5")]
    pub md5: Md5Digest,
}
