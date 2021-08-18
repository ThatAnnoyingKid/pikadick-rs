/// A list of deleted images
#[derive(serde::Deserialize, Debug)]
pub struct DeletedImagesList {
    /// A list of deleted posts
    #[serde(rename = "post", default)]
    pub posts: Vec<Post>,
}

/// A deleted post
#[derive(serde::Deserialize, Debug)]
pub struct Post {
    /// The deleted post id
    pub deleted: u64,
    /// The md5 of the deleted post
    pub md5: String,
}
