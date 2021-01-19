use std::collections::HashMap;
use url::Url;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status")]
pub enum GetVideoResponse {
    #[serde(rename = "ok")]
    Ok(GetVideoResponseOk),

    #[serde(rename = "error")]
    Error(GetVideoResponseError),
}

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

#[derive(Debug, serde::Deserialize)]
pub struct VideoData {
    pub author: String,
    pub duration: u64,
    pub has_audio: bool,
    pub is_gif: bool,
    pub is_video: bool,

    #[serde(rename = "type")]
    pub kind: String,

    pub nsfw: bool,
    pub provider_name: Option<String>,
    pub subreddit: String,
    pub thumbnail: Url,
    pub title: String,
    pub url: Url,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GetVideoResponseError {
    pub errores: Option<HashMap<String, String>>,
    pub meme: Option<String>,
    pub msg: Option<String>,

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
