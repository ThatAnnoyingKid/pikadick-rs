use super::Md5Digest;
use std::num::NonZeroU64;

/// A list of deleted images
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct DeletedImageList {
    /// A list of deleted posts
    #[serde(alias = "post", default)]
    pub posts: Box<[Post]>,
}

/// A deleted post
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Post {
    /// The deleted post id
    #[serde(alias = "@deleted")]
    pub deleted: NonZeroU64,

    /// The md5 hash of the deleted post.
    ///
    /// This can be None sometimes for an unknown reason.
    #[serde(alias = "@md5", with = "serde_md5_digest")]
    pub md5: Option<Md5Digest>,
}

mod serde_md5_digest {
    use super::*;
    use serde::{
        de::Error,
        Serialize,
    };
    use std::borrow::Cow;

    pub(super) fn deserialize<'de, D>(deserialize: D) -> Result<Option<Md5Digest>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize;

        let value = Cow::<str>::deserialize(deserialize)?;
        if value.is_empty() {
            return Ok(None);
        }

        let value = value.parse().map_err(D::Error::custom)?;

        Ok(Some(value))
    }

    pub(super) fn serialize<S>(value: &Option<Md5Digest>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) => value.serialize(serializer),
            None => serializer.serialize_str(""),
        }
    }
}
