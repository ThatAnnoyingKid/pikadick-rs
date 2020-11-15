use std::collections::HashMap;
use url::Url;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Stat {
    pub metadata: Metadata,
    pub value: f64,
    pub percentile: Option<f32>,
    pub rank: Option<u32>,

    #[serde(rename = "displayValue")]
    pub display_value: String,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Stat {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn icon_url(&self) -> Option<&Url> {
        self.metadata.icon_url.as_ref()
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    pub key: String,
    pub name: String,

    #[serde(rename = "categoryKey")]
    pub category_key: String,

    #[serde(rename = "categoryName")]
    pub category_name: String,

    pub description: Option<String>,

    #[serde(rename = "isReversed")]
    pub is_reversed: bool,

    #[serde(rename = "iconUrl")]
    pub icon_url: Option<Url>,

    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
