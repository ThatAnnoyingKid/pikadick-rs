use anyhow::Context;
use std::collections::HashMap;

const CARGO_TOML_CONFIG_METADATA_KEY: &str = "rpi-deploy";
const FILE_CONFIG_FILE_NAME: &str = "rpi-deploy.toml";

/// Config for this tool in Cargo.toml
#[derive(serde::Deserialize)]
pub struct CargoTomlConfig {
    /// Targets to build
    pub targets: Vec<String>,
}

impl CargoTomlConfig {
    /// Load this from a given cargo_metadata package
    pub fn load_from_package(package: &cargo_metadata::Package) -> anyhow::Result<Self> {
        let value = package
            .metadata
            .get(CARGO_TOML_CONFIG_METADATA_KEY)
            .with_context(|| {
                let package_name = package.name.as_str();
                format!(
                    "missing `{CARGO_TOML_CONFIG_METADATA_KEY}` key in metadata for `{package_name}`"
                )
            })?;

        serde_json::from_value(value.clone())
            .context("failed to parse `Cargo.toml` metadata config")
    }
}

/// The config file
#[derive(serde::Deserialize)]
pub struct FileConfig {
    /// Machine configs
    #[serde(flatten)]
    pub machines: HashMap<String, MachineConfig>,
}

impl FileConfig {
    /// Load a file config.
    pub fn new() -> anyhow::Result<Self> {
        let config_str = match std::fs::read_to_string(FILE_CONFIG_FILE_NAME) {
            Ok(config_str) => config_str,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("failed to read file config at `{}`", FILE_CONFIG_FILE_NAME)
                })?;
            }
        };

        toml::from_str(&config_str).context("failed to parse file config")
    }

    /// Make an empty config.
    fn empty() -> Self {
        Self {
            machines: HashMap::new(),
        }
    }

    /// Get the machine config for a key.
    pub fn get_machine_config(&self, name: &str) -> Option<&MachineConfig> {
        self.machines.get(name)
    }
}

/// The config for deploying to a host
#[derive(serde::Deserialize)]
pub struct MachineConfig {
    /// The ssh address
    pub address: String,

    /// ssh username
    pub username: String,

    /// ssh password
    pub password: String,

    /// The target
    pub target: String,
}
