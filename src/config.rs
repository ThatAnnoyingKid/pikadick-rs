use anyhow::Context;
use serde::{
    Deserialize,
    Serialize,
};
use serenity::{
    model::prelude::GuildId,
    utils::validate_token,
};
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

/// The bot config
#[derive(Deserialize, Debug)]
pub struct Config {
    /// The discord token
    pub token: String,

    /// The application id
    pub application_id: u64,

    /// Prefix for the bot
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// Status config
    pub status: Option<StatusConfig>,

    /// Data dir
    pub data_dir: PathBuf,

    /// The test guild
    pub test_guild: Option<GuildId>,

    /// FML config
    pub fml: FmlConfig,

    /// DeviantArt config
    pub deviantart: DeviantArtConfig,

    /// The log config
    #[serde(default)]
    pub log: LogConfig,

    /// Unknown extra data
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

/// FML config
#[derive(Deserialize, Debug)]
pub struct FmlConfig {
    /// FML API key
    pub key: String,

    /// Unknown extra data
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

/// Deviant Config
#[derive(Deserialize, Debug)]
pub struct DeviantArtConfig {
    /// Username
    pub username: String,

    /// Password
    pub password: String,

    /// Unknown extra data
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

/// Log Config
#[derive(Deserialize, Debug)]
pub struct LogConfig {
    /// The logging directives.
    #[serde(default = "LogConfig::default_directives")]
    pub directives: Vec<String>,

    /// Whether to use open telemetry
    #[serde(default)]
    pub use_opentelemetry: bool,

    /// The OTLP endpoint
    pub endpoint: Option<String>,

    /// Headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

impl LogConfig {
    /// If logging directives not given, choose some defaults.
    fn default_directives() -> Vec<String> {
        // Only enable pikadick since serenity likes puking in the logs during connection failures
        // serenity's framework section seems ok as well
        vec![
            "pikadick=info".to_string(),
            "serenity::framework::standard=info".to_string(),
        ]
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            directives: Self::default_directives(),

            use_opentelemetry: false,
            endpoint: None,
            headers: HashMap::new(),
        }
    }
}

impl Config {
    /// Shortcut for getting the status name
    pub fn status_name(&self) -> Option<&str> {
        self.status.as_ref().map(|s| s.name.as_str())
    }

    /// Shortcut for getting the status url
    pub fn status_url(&self) -> Option<&str> {
        self.status.as_ref().and_then(|s| s.url.as_deref())
    }

    /// Shortcut for getting the status type
    pub fn status_type(&self) -> Option<ActivityKind> {
        self.status.as_ref().and_then(|s| s.kind)
    }

    /// The log file dir
    pub fn log_file_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// The cache dir
    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    /// Load a config from a path
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        std::fs::read(path)
            .with_context(|| format!("failed to read config from '{}'", path.display()))
            .and_then(|b| Self::load_from_bytes(&b))
    }

    /// Load a config from bytes
    pub fn load_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        toml::from_slice(bytes).context("failed to parse config")
    }

    /// Validate a config
    pub fn validate(&mut self) -> Vec<ValidationMessage> {
        let mut errors = Vec::with_capacity(3);

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

#[derive(Deserialize, Debug)]
pub struct StatusConfig {
    #[serde(rename = "type")]
    #[serde(default)]
    kind: Option<ActivityKind>,
    name: String,
    url: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
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

/// Validation Errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid token")]
    InvalidToken,
    #[error("missing status type")]
    MissingStatusType,
    #[error("missing stream url type")]
    MissingStreamUrl,

    #[error("{0}")]
    Generic(Cow<'static, str>),
}

#[derive(Copy, Clone, Debug)]
pub enum Severity {
    Warn,
    Error,
}
