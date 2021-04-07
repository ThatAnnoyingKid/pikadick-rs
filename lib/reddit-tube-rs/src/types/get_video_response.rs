use std::collections::HashMap;
use url::Url;

/// The response for getting a video
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status")]
pub enum GetVideoResponse {
    #[serde(rename = "ok")]
    Ok(GetVideoResponseOk),

    #[serde(rename = "error")]
    Error(GetVideoResponseError),
}

/// A good video response
#[derive(Debug, serde::Deserialize)]
pub struct GetVideoResponseOk {
    pub affected: i64,
    pub already_downloaded: bool,
    pub file_hash: String,
    pub meme: String,
    pub meme_msg: String,
    pub points: i64,
    pub thumbnail_name: String,
    pub share_url: Url,
    pub short_id: String,
    pub url: Url,
    pub user_email: String,
    pub user_hash: String,
    pub video_data: Option<VideoData>,
    pub video_size: String,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
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
    pub kind: String,

    /// Whether the post is nsfw
    pub nsfw: bool,
    /// ?
    pub provider_name: Option<String>,
    /// The subreddit
    pub subreddit: String,
    /// The thumbnail url
    pub thumbnail: Url,
    /// The post title
    pub title: String,
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
    pub errores: Option<HashMap<String, String>>,
    /// Meme
    pub meme: Option<String>,
    /// Error message
    pub msg: Option<String>,

    /// Unknown data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl std::fmt::Display for GetVideoResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.msg {
            Some(msg) => msg.fmt(f),
            None => "failed to get video response".fmt(f),
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
