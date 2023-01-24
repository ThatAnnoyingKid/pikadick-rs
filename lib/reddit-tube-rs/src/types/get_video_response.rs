use std::collections::HashMap;
use url::Url;

/// The response for getting a video
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status")]
pub enum GetVideoResponse {
    #[serde(rename = "ok")]
    Ok(Box<GetVideoResponseOk>),

    #[serde(rename = "error")]
    Error(GetVideoResponseError),
}

impl GetVideoResponse {
    /// Transform this into a result
    pub fn into_result(self) -> Result<Box<GetVideoResponseOk>, GetVideoResponseError> {
        match self {
            Self::Ok(ok) => Ok(ok),
            Self::Error(err) => Err(err),
        }
    }
}

/// A good video response
#[derive(Debug, serde::Deserialize)]
pub struct GetVideoResponseOk {
    /// ?
    pub affected: i32,

    /// ?
    pub already_downloaded: bool,

    /// The file hash?
    pub file_hash: Box<str>,

    /// ?
    pub meme: Box<str>,
    /// ?
    pub meme_msg: Box<str>,
    pub points: i64,
    pub thumbnail_name: Box<str>,
    pub share_url: Url,
    pub short_id: Box<str>,
    pub url: Url,
    pub user_email: Box<str>,
    pub user_hash: Box<str>,

    /// The video data, if it exists.
    pub video_data: Option<Box<VideoData>>,

    /// The size of the video as a human-readable string
    ///
    /// Example: 614.25KB
    pub video_size: Box<str>,
}

/// Video Data
#[derive(Debug, serde::Deserialize)]
pub struct VideoData {
    /// Author
    pub author: String,
    /// Video duration
    pub duration: u64,
    /// Whether the post has audio
    pub has_audio: bool,
    /// Whether the post is a gif
    pub is_gif: bool,
    /// Whether the post is a video
    pub is_video: bool,

    /// The post type?
    #[serde(rename = "type")]
    pub kind: Box<str>,

    /// Whether the post is nsfw
    pub nsfw: bool,
    /// ?
    pub provider_name: Option<Box<str>>,
    /// The subreddit
    pub subreddit: Box<str>,
    /// The thumbnail url
    pub thumbnail: Url,
    /// The post title
    pub title: Box<str>,
    /// The post url?
    pub url: Url,

    /// Unknown data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Error video response
#[derive(Debug, serde::Deserialize)]
pub struct GetVideoResponseError {
    /// Errors
    #[serde(rename = "errores")]
    pub errors: Option<HashMap<String, String>>,
    /// Meme
    pub meme: Option<Box<str>>,
    /// Error message
    pub msg: Option<Box<str>>,
}

impl std::fmt::Display for GetVideoResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.msg, &self.errors) {
            (Some(msg), _) => msg.fmt(f),
            (_, Some(errors)) => errors
                .values()
                .next()
                .map(|s| s.as_str())
                .unwrap_or("failed to get video response")
                .fmt(f),
            (None, None) => "failed to get video response".fmt(f),
        }
    }
}

impl std::error::Error for GetVideoResponseError {}

#[cfg(test)]
mod test {
    use super::*;

    const VALID_1: &str = include_str!("../../test_data/valid_get_video1.json");
    const VALID_2: &str = include_str!("../../test_data/valid_get_video2.json");
    const INVALID_CSRF: &str = include_str!("../../test_data/invalid_csrf_get_video.json");
    const INVALID_POST_TYPE: &str =
        include_str!("../../test_data/invalid_post_type_get_video.json");

    #[test]
    fn parse_valid_1_get_video_response() {
        let res: GetVideoResponse = serde_json::from_str(VALID_1).unwrap();
        dbg!(res);
    }

    #[test]
    fn parse_valid_2_get_video_response() {
        let res: GetVideoResponse = serde_json::from_str(VALID_2).unwrap();
        dbg!(res);
    }

    #[test]
    fn parse_invalid_csrf_get_video_response() {
        let res: GetVideoResponse = serde_json::from_str(INVALID_CSRF).unwrap();
        dbg!(res);
    }

    #[test]
    fn parse_invalid_post_type_get_video_response() {
        let res: GetVideoResponse = serde_json::from_str(INVALID_POST_TYPE).unwrap();
        dbg!(res);
    }
}
