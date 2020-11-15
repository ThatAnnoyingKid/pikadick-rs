use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    Err {
        error: String,
    },
    Ok {
        data: T,
        #[serde(flatten)]
        unknown: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Article {
    pub apikey: Option<String>,
    pub area: Option<String>,
    pub author: String,
    pub bitly: Option<String>,
    pub city: Option<String>,
    pub content: String,
    pub content_hidden: String,
    pub country: Option<String>,
    pub countrycode: Option<String>,
    pub created: String,
    pub flag: u32,
    pub gender: Option<u8>,
    pub id: u64,
    pub images: Vec<ArticleImage>,
    pub ip: Option<String>,
    pub keywords: Vec<ArticleKeyword>,
    pub layout: u32,
    pub metrics: ArticleMetrics,
    pub openview: u32,
    pub origin: Option<String>,
    pub paragraphs: Vec<serde_json::Value>,
    pub published: String,
    pub site: u32,
    pub siteorig: Option<serde_json::Value>,
    pub slug: String,
    #[serde(rename = "socialTruncate")]
    pub social_truncate: bool,
    pub spicy: bool,
    pub status: u32,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub article_type: u32,
    pub updated: String,
    pub url: String,
    pub user: u64,
    pub usermetrics: ArticleUsermetrics,
    pub videos: Vec<serde_json::Value>,
    pub vote: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleImage {
    pub copyright: Option<String>,
    pub height: u32,
    pub legend: Option<serde_json::Value>,
    pub name: String,
    pub url: String,
    pub usage: u32,
    pub width: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleKeyword {
    pub label: String,
    pub rub: bool,
    pub uid: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleMetrics {
    pub article: u64,
    pub comment: u32,
    pub favorite: u32,
    pub mod_negative: u32,
    pub mod_positive: u32,
    pub reports: u32,
    pub smiley_amusing: u32,
    pub smiley_funny: u32,
    pub smiley_hilarious: u32,
    pub smiley_weird: u32,
    pub votes_down: u32,
    pub votes_up: u32,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleUsermetrics {
    pub favorite: bool,
    pub smiley: Option<serde_json::Value>,
    pub votes: Option<serde_json::Value>,
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
