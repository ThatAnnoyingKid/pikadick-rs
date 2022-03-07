use anyhow::Context;
use std::{
    collections::HashMap,
    path::Path,
};

/// The config file
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// The targets for which config is provided.
    #[serde(flatten)]
    pub targets: HashMap<String, ConfigTarget>,
}

/// A target in the config
#[derive(Debug, serde::Deserialize)]
pub struct ConfigTarget {
    /// The linker exe name.
    /// Example: "arm-linux-gnueabihf-gcc"
    pub linker: String,

    /// The contents of s cmake toolchain file
    pub cmake_toolchain_file: Option<String>,

    /// Env vars set for this target.
    ///
    /// Example PERL = "C:/Users/username/scoop/apps/msys2/current/usr/bin/perl"
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl Config {
    /// Load a config from a path
    pub fn load_from_path(config_path: &Path) -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string(&config_path).context("failed to read config")?;
        toml::from_str(&config_str).context("failed to parse config")
    }
}
