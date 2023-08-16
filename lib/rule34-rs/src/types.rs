/// The [`DeletedImageList`] type
pub mod deleted_image_list;
/// The [`HtmlPost`] type
#[cfg(feature = "scrape")]
pub mod html_post;
/// The [`PostList`] type
pub mod post_list;
/// The [`TagList`] type
pub mod tag_list;

#[cfg(feature = "scrape")]
pub use self::html_post::HtmlPost;
pub use self::{
    deleted_image_list::DeletedImageList,
    post_list::{
        Post,
        PostList,
        Rating,
    },
    tag_list::{
        Tag,
        TagKind,
        TagList,
    },
};
