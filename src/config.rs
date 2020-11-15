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

#[derive(Debug)]
pub enum ConfigError<'a> {
    DoesNotExist(&'a Path),
    IsNotFile(&'a Path),

    Io(std::io::Error),
    TomlDe(toml::de::Error),
}

impl std::fmt::Display for ConfigError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::DoesNotExist(path) => write!(f, "{} does not exist", path.display()),
            ConfigError::IsNotFile(path) => write!(f, "{} is not a file", path.display()),
            ConfigError::Io(e) => e.fmt(f),
            ConfigError::TomlDe(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ConfigError<'_> {}

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

    pub fn load_from_path(p: &Path) -> Result<Self, ConfigError<'_>> {
        if !p.exists() {
            return Err(ConfigError::DoesNotExist(p));
        }

        if !p.is_file() {
            return Err(ConfigError::IsNotFile(p));
        }

        std::fs::read(p)
            .map_err(ConfigError::Io)
            .and_then(|b| Self::load_from_bytes(&b))
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, ConfigError<'static>> {
        toml::from_slice(bytes).map_err(ConfigError::TomlDe)
    }

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
