use crate::{
    Error,
    FmlResult,
};
use serde::Deserialize;
use std::collections::HashMap;

/// An API Response
#[derive(Deserialize)]
pub struct ApiResponse<T> {
    /// A potential API error.
    ///
    /// Populated on error.
    pub error: Option<String>,

    /// A potential response payload.
    ///
    /// Populated if successful.
    pub data: Option<T>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl<T> ApiResponse<T> {
    /// Whether the response is an error.
    ///
    /// This performs a check on the error field.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Whether the response is a success.
    ///
    /// This performs a check on the data field.
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.data.is_some()
    }

    /// Checks whether the data contained is valid.
    ///
    /// This looks to see if this is both an error and success or neither.
    pub fn is_valid_response(&self) -> bool {
        self.is_error() || self.is_success()
    }
}

impl<T> From<ApiResponse<T>> for FmlResult<T> {
    fn from(response: ApiResponse<T>) -> Self {
        match (response.data, response.error) {
            (Some(_data), Some(_e)) => Err(Error::InvalidApiResponse),
            (Some(data), None) => Ok(data),
            (None, Some(e)) => Err(Error::Api(e)),
            (None, None) => Err(Error::InvalidApiResponse),
        }
    }
}

/// An FML article
#[derive(Debug, Deserialize)]
pub struct Article {
    pub apikey: Option<String>,
    pub area: Option<String>,
    pub author: Option<String>,
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

#[cfg(test)]
mod test {
    use super::*;

    const DATA_1: &str = include_str!("../test_data/data_1.json");

    #[test]
    fn data_1() {
        let _data_1: ApiResponse<Vec<Article>> =
            serde_json::from_str(DATA_1).expect("failed to parse");
    }
}
