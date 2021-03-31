use anyhow::Context;
use serde::{
    Deserialize,
    Serialize,
};
use serenity::client::validate_token;
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{
        Path,
        PathBuf,
    },
};

fn default_prefix() -> String {
    "p!".to_string()
}

#[derive(Deserialize, Debug)]
pub struct Config {
    token: String,

    #[serde(default = "default_prefix")]
    prefix: String,

    status: Option<StatusConfig>,

    data_dir: PathBuf,

    fml: FmlConfig,

    #[serde(flatten)]
    extra: HashMap<String, toml::Value>,
}

#[derive(Deserialize, Debug)]
pub struct FmlConfig {
    key: String,

    #[serde(flatten)]
    extra: HashMap<String, toml::Value>,
}

impl Config {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn status_name(&self) -> Option<&str> {
        self.status.as_ref().map(|s| s.name.as_str())
    }

    pub fn status_url(&self) -> Option<&str> {
        self.status.as_ref().and_then(|s| s.url.as_deref())
    }

    pub fn status_type(&self) -> Option<ActivityKind> {
        self.status.as_ref().and_then(|s| s.kind)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn fml(&self) -> &FmlConfig {
        &self.fml
    }

    /// Load a config from a path
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        std::fs::read(path)
            .with_context(|| format!("Failed to read config from '{}'", path.display()))
            .and_then(|b| Self::load_from_bytes(&b))
    }

    /// Load a config from bytes
    pub fn load_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        toml::from_slice(bytes).context("Failed to parse config")
    }

    /// Validate a config
    pub fn validate(&mut self) -> Vec<ValidationMessage> {
        let mut errors = Vec::new();

        if let Err(_e) = validate_token(&self.token) {
            errors.push(ValidationMessage {
                severity: Severity::Error,
                error: ValidationError::InvalidToken,
            });
        }

        if let Some(config) = &self.status {
            if let (Some(ActivityKind::Streaming), None) = (config.kind, &config.url) {
                errors.push(ValidationMessage {
                    severity: Severity::Error,
                    error: ValidationError::MissingStreamUrl,
                });
            }

            if let (None, _) = (config.kind, &config.url) {
                errors.push(ValidationMessage {
                    severity: Severity::Warn,
                    error: ValidationError::MissingStatusType,
                });
            }
        }

        errors
    }
}

impl FmlConfig {
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Deserialize, Debug)]
struct StatusConfig {
    #[serde(rename = "type")]
    #[serde(default)]
    kind: Option<ActivityKind>,
    name: String,
    url: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub enum ActivityKind {
    Listening,
    Playing,
    Streaming,
}

impl Default for ActivityKind {
    fn default() -> Self {
        ActivityKind::Playing
    }
}

#[derive(Debug)]
pub struct ValidationMessage {
    severity: Severity,
    error: ValidationError,
}

impl ValidationMessage {
    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn error(&self) -> &ValidationError {
        &self.error
    }
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidToken,
    MissingStatusType,
    MissingStreamUrl,

    #[allow(dead_code)]
    Generic(Cow<'static, str>),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidToken => write!(f, "Invalid Token"),
            ValidationError::MissingStatusType => write!(f, "Missing Status Type"),
            ValidationError::MissingStreamUrl => write!(f, "Missing Stream Url"),
            ValidationError::Generic(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Copy, Clone, Debug)]
pub enum Severity {
    Warn,
    Error,
}
