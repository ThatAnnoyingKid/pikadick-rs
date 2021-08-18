use std::collections::HashMap;
use url::Url;

/// A result entry
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ResultEntry {
    /// Header
    pub header: Header,
    /// Data
    pub data: Data,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A result entry header
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Header {
    /// Result similarity
    pub similarity: String,
    /// Image thumbnail
    pub thumbnail: Url,
    /// index id?
    pub index_id: u64,
    /// The index name
    pub index_name: String,
    /// the # of dupes
    pub dupes: u64,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Result data
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Data {
    /// ?
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ext_urls: Vec<Url>,
    /// title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Deviantart ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub da_id: Option<String>,
    /// Author name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// Author URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<Url>,
    /// Pixiv id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixiv_id: Option<u64>,
    /// member name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    /// member id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_id: Option<u64>,
    /// source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// ?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anidb_aid: Option<u64>,
    ///?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// anime year?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// ?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub est_time: Option<String>,
    /// fur affinity id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fa_id: Option<u64>,
    /// twitter tweet creation date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// tweet_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tweet_id: Option<String>,
    /// twitter_user_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_user_id: Option<String>,
    /// twitter_user_handle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_user_handle: Option<String>,
    /// creator?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<Creator>,
    /// eng name?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eng_name: Option<String>,
    /// jp name?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jp_name: Option<String>,
    /// bcy_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcy_id: Option<u64>,
    /// member_link_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_link_id: Option<u64>,
    /// bcy_type?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcy_type: Option<String>,
    /// fn_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_id: Option<u64>,
    /// fn_type?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_type: Option<String>,
    /// pawoo_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pawoo_id: Option<u64>,
    /// pawoo_user_acct?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pawoo_user_acct: Option<String>,
    /// pawoo_user_username?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pawoo_user_username: Option<String>,
    /// pawoo_user_display_name?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pawoo_user_display_name: Option<String>,
    /// seiga_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seiga_id: Option<u64>,
    /// danbooru_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danbooru_id: Option<u64>,
    /// material?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    /// characters?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters: Option<String>,
    /// konachan_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub konachan_id: Option<u64>,
    /// drawr_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawr_id: Option<u64>,
    /// gelbooru_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gelbooru_id: Option<u64>,
    /// sankaku_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sankaku_id: Option<u64>,
    /// artist?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// author?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// md_id?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md_id: Option<u64>,

    /// Extra K/Vs
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// The creator field of [`Data`]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Creator {
    /// a single creator
    Single(String),

    /// multiple creators
    Multiple(Vec<String>),
}
