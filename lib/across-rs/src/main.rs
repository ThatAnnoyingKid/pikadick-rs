use anyhow::{
    bail,
    ensure,
    Context,
};
use std::{
    collections::HashMap,
    fmt::Write,
    io::Write as _,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};
use tempfile::NamedTempFile;

#[derive(argh::FromArgs)]
#[argh(description = "a tool to help in cross compilation")]
struct Options {
    #[argh(option, description = "the target")]
    target: String,

    #[argh(option, description = "the features")]
    features: Option<String>,

    #[argh(switch, description = "whether to build in release")]
    release: bool,

    #[argh(switch, long = "vv", description = "very verbose")]
    very_verbose: bool,

    #[argh(switch, description = "whether to run strip on the binary")]
    use_strip: bool,

    #[argh(
        option,
        short = 'c',
        default = "PathBuf::from(\"./cross-compile-info.toml\")",
        description = "the path to the config file"
    )]
    config: PathBuf,
}

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
    pub cmake_toolchain: Option<String>,

    /// Env vars set for this target.
    ///
    /// Example PERL = "C:/Users/username/scoop/apps/msys2/current/usr/bin/perl"
    pub env: HashMap<String, String>,
}

impl Config {
    /// Load a config from a path
    pub fn load_from_path(config_path: &Path) -> anyhow::Result<Self> {
        let config_str = std::fs::read_to_string(&config_path).context("failed to read config")?;
        toml::from_str(&config_str).context("failed to parse config")
    }
}

/// The entry point
fn main() {
    let options: Options = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };

    std::process::exit(code);
}

/// The real entry point
fn real_main(options: Options) -> anyhow::Result<()> {
    let config_path = options
        .config
        .canonicalize()
        .context("failed to canonicalize config path")?;
    println!("# loading `{}`...", config_path.display());
    let config = Config::load_from_path(&config_path).context("failed to load config")?;

    // Get target config
    let target_config = config
        .targets
        .get(options.target.as_str())
        .context("missing config for target")?;

    let mut rust_flags = String::with_capacity(64);
    write!(&mut rust_flags, "-Clinker={} ", target_config.linker)?;
    if options.use_strip {
        // TODO: allow user to specify strip level
        write!(&mut rust_flags, "-Cstrip=symbols ")?;
    }

    let mut envs = target_config.env.clone();
    envs.insert("RUSTFLAGS".to_string(), rust_flags);
    // TODO: Make configurable
    envs.insert("RUST_BACKTRACE".to_string(), "1".to_string());

    // Generate cmake toolchain file if needed
    // TODO: Allow caching in specialized dir
    let _cmake_toolchain_path =
        if let Some(cmake_toolchain_str) = target_config.cmake_toolchain.as_ref() {
            let mut cmake_toolchain =
                NamedTempFile::new().context("failed to create cmake toolchain file")?;
            cmake_toolchain
                .write_all(cmake_toolchain_str.as_bytes())
                .context("failed to write cmake toolchain contents")?;
            cmake_toolchain
                .as_file()
                .sync_all()
                .context("failed to sync cmake toolchain contents")?;

            let path = cmake_toolchain.into_temp_path();
            let path_str = path
                .to_str()
                .context("cmake_toolchain file path is not valid unicode")?;
            let value = envs.insert("CMAKE_TOOLCHAIN".to_string(), path_str.to_string());
            if let Some(value) = value {
                bail!(
                    "`CMAKE_TOOLCHAIN` is already specified in the environment with value `{}`",
                    value
                );
            }

            // Keep alive to ensure file lives until compilation finishes
            Some(path)
        } else {
            None
        };

    let mut command = Command::new("cargo");
    command.args(&["build", "--target", options.target.as_str()]);
    if let Some(features) = options.features.as_ref() {
        command.args(&["--features", features]);
    }
    if options.release {
        command.arg("--release");
    }
    if options.very_verbose {
        command.arg("-vv");
    }
    command.envs(envs.iter());

    println!("# compiling...");
    let status = command.status().context("failed to call compile command")?;
    let code = status.code();
    ensure!(
        status.success(),
        "compile command exited with status {}",
        if let Some(code) = code.as_ref() {
            code as &dyn std::fmt::Display
        } else {
            &"unknown" as &dyn std::fmt::Display
        }
    );

    Ok(())
}
