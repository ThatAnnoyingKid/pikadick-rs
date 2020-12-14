use crate::{
    InstaError,
    InstaResult,
};
use reqwest::Url;
use select::{
    document::Document,
    predicate::{
        And,
        Attr,
        Name,
    },
};
use std::str::FromStr;

/// Get a meta value by name.
/// Example meta tag: `<meta property="og:video:height" content="640" />`
fn get_meta_prop<'a>(doc: &'a Document, key: &str) -> Option<&'a str> {
    doc.find(And(Name("meta"), Attr("property", key)))
        .next()
        .and_then(|el| el.attr("content"))
}

/// An Instagram Post
#[derive(Debug)]
pub struct Post {
    /// The kind of post
    pub kind: String,

    /// Data unique for video posts
    pub video_data: Option<VideoData>,
}

impl Post {
    /// Make a post from a doc
    pub(crate) fn from_doc(doc: &Document) -> InstaResult<Self> {
        let kind = get_meta_prop(doc, "og:type")
            .ok_or(InstaError::MissingElement("meta && property=og:type"))?
            .to_string();

        let video_data = if kind == "video" {
            Some(VideoData::from_doc(&doc)?)
        } else {
            None
        };

        Ok(Post { kind, video_data })
    }
}

/// Data unique for video posts
#[derive(Debug)]
pub struct VideoData {
    /// Video Url
    pub video_url: Url,

    /// Secure video url
    pub secure_video_url: Url,

    /// Video type
    pub video_type: String,

    /// Video width
    pub video_width: u32,

    /// Video height
    pub video_height: u32,
}

impl VideoData {
    /// Get video data from a doc
    pub(crate) fn from_doc(doc: &Document) -> InstaResult<Self> {
        let video_url = get_meta_prop(doc, "og:video")
            .and_then(|s| Url::parse(s).ok())
            .ok_or(InstaError::MissingElement("meta && property=og:video"))?;

        let secure_video_url = get_meta_prop(doc, "og:video:secure_url")
            .and_then(|s| Url::parse(s).ok())
            .ok_or(InstaError::MissingElement(
                "meta && property=og:video:secure_url",
            ))?;

        let video_type = get_meta_prop(doc, "og:video:type")
            .ok_or(InstaError::MissingElement("meta && property=og:video:type"))?
            .to_string();

        let video_width = get_meta_prop(doc, "og:video:width")
            .and_then(|s| u32::from_str(s).ok())
            .ok_or(InstaError::MissingElement(
                "meta && property=og:video:width",
            ))?;

        let video_height = get_meta_prop(doc, "og:video:height")
            .and_then(|s| u32::from_str(s).ok())
            .ok_or(InstaError::MissingElement(
                "meta && property=og:video:height",
            ))?;

        Ok(VideoData {
            video_url,
            secure_video_url,
            video_type,
            video_width,
            video_height,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const VIDEO_POST: &str = include_str!("../test_data/video_post.html");

    #[test]
    fn parse_video_post() {
        let doc = Document::from(VIDEO_POST);
        let post = Post::from_doc(&doc).unwrap();
        dbg!(post);
    }
}
