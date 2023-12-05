/// The [`DeletedImageList`] type
pub mod deleted_image_list;
/// The [`HtmlPost`] type
#[cfg(feature = "scrape")]
pub mod html_post;
/// An md5 Digest
pub mod md5_digest;
/// The [`NoteList`] type
pub mod note_list;
/// The [`PostList`] type
pub mod post_list;
/// The [`TagList`] type
pub mod tag_list;

#[cfg(feature = "scrape")]
pub use self::html_post::HtmlPost;
pub use self::{
    deleted_image_list::DeletedImageList,
    md5_digest::Md5Digest,
    note_list::{
        Note,
        NoteList,
    },
    post_list::{
        Post,
        PostList,
        PostStatus,
        Rating,
    },
    tag_list::{
        Tag,
        TagKind,
        TagList,
    },
};
