mod config;

pub use self::config::Config;
use anyhow::{
    bail,
    ensure,
    Context,
};
use cargo_metadata::MetadataCommand;
use std::{
    collections::HashMap,
    fs::File,
    io::Write as _,
    path::PathBuf,
    process::Command,
};

#[derive(argh::FromArgs)]
#[argh(description = "a tool to help in cross compilation")]
struct Options {
    #[argh(option, description = "the target")]
    target: String,

    #[argh(option, description = "the features")]
    features: Option<String>,

    #[argh(switch, description = "whether to build in release")]
    release: bool,

    #[argh(option, description = "the build profile")]
    profile: Option<String>,

    #[argh(switch, long = "vv", description = "very verbose")]
    very_verbose: bool,

    #[argh(
        option,
        long = "cargo-build-wrapper",
        description = "use this command instead of `cargo build`"
    )]
    cargo_build_wrapper: Option<String>,

    #[argh(
        option,
        short = 'c',
        default = "PathBuf::from(\"./cross-compile-info.toml\")",
        description = "the path to the config file"
    )]
    config: PathBuf,
}

/// The entry point
fn main() -> anyhow::Result<()> {
    let options: Options = argh::from_env();
    real_main(options)?;
    Ok(())
}

/// The real entry point
fn real_main(options: Options) -> anyhow::Result<()> {
    ensure!(
        !(options.release && options.profile.is_some()),
        "the `--release` and `--profile` flags aure mutually exclusive"
    );

    println!("Fetching cargo metadata...");
    let metadata = MetadataCommand::new()
        .exec()
        .context("failed to get cargo metadata")?;

    // Make across dir in target to cache files
    let across_dir = metadata.target_directory.join("across");
    std::fs::create_dir_all(&across_dir).context("failed to create across directory")?;

    let config_path = options
        .config
        .canonicalize()
        .context("failed to canonicalize config path")?;
    println!("Loading `{}`...", config_path.display());
    let config = Config::load_from_path(&config_path).context("failed to load config")?;

    // Get target config
    let target_config = config
        .targets
        .get(options.target.as_str())
        .context("missing config for target")?;

    let profile = if options.release {
        Some("release")
    } else {
        options.profile.as_deref()
    };

    // Setup command builder
    let mut command_builder = CrossCompileCommandBuilder::new();
    command_builder
        .target(options.target.as_str())
        .linker(target_config.linker.as_str())
        .very_verbose(options.very_verbose);

    if let Some(profile) = profile {
        command_builder.profile(profile);
    }

    if let Some(cargo_build_wrapper) = options.cargo_build_wrapper {
        command_builder.cargo_build_wrapper(
            cargo_build_wrapper
                .split(' ')
                .filter(|t| !t.is_empty())
                .map(Box::from)
                .collect(),
        );
    }

    if let Some(features) = options.features.as_deref() {
        command_builder.features(features);
    }

    let mut envs = target_config.env.clone();

    // Generate cmake toolchain file if needed
    if let Some(cmake_toolchain_file_str) = target_config.cmake_toolchain_file.as_ref() {
        let cmake_toolchain_file_path =
            across_dir.join(format!("{}.cmake", options.target.as_str()));
        let mut cmake_toolchain_file = File::create(&cmake_toolchain_file_path)
            .context("failed to create cmake toolchain file")?;
        cmake_toolchain_file
            .write_all(cmake_toolchain_file_str.as_bytes())
            .context("failed to write cmake toolchain file contents")?;
        cmake_toolchain_file
            .sync_all()
            .context("failed to sync cmake toolchain file contents")?;

        let value = envs.insert(
            "CMAKE_TOOLCHAIN_FILE".to_string(),
            cmake_toolchain_file_path.to_string(),
        );
        if let Some(value) = value {
            bail!(
                "`CMAKE_TOOLCHAIN_FILE` is already specified in the environment with value `{value}`"
            );
        }
    }

    for (key, value) in envs.iter() {
        command_builder.environment_variable(key.as_str(), value.as_str());
    }

    let mut command = command_builder
        .build_command()
        .context("failed to build command")?;

    println!("Compiling...");
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

/// A builder for cross compile commands
pub struct CrossCompileCommandBuilder {
    /// The target triple
    pub target: Option<Box<str>>,

    /// Build features
    pub features: Option<Box<str>>,

    /// The profile
    pub profile: Option<Box<str>>,

    /// Whether to set the very verbose flag
    pub very_verbose: bool,

    /// Environment Variables
    pub environment_variables: HashMap<Box<str>, Box<str>>,

    /// The linker
    pub linker: Option<Box<str>>,

    /// The cargo build wrapper
    pub cargo_build_wrapper: Option<Box<[Box<str>]>>,

    /// This is some if the builder errored.
    pub error: Option<anyhow::Error>,
}

impl CrossCompileCommandBuilder {
    /// A cross compile command
    pub fn new() -> Self {
        Self {
            target: None,
            features: None,
            profile: None,
            very_verbose: false,
            environment_variables: HashMap::with_capacity(16),
            linker: None,
            cargo_build_wrapper: None,

            error: None,
        }
    }

    /// Set the target
    pub fn target(&mut self, target: impl Into<Box<str>>) -> &mut Self {
        self.target = Some(target.into());
        self
    }

    /// Set the build features
    pub fn features(&mut self, features: impl Into<Box<str>>) -> &mut Self {
        self.features = Some(features.into());
        self
    }

    /// Set the profile
    pub fn profile(&mut self, profile: impl Into<Box<str>>) -> &mut Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set the very verbose flag
    pub fn very_verbose(&mut self, very_verbose: bool) -> &mut Self {
        self.very_verbose = very_verbose;
        self
    }

    /// Add a single env var
    pub fn environment_variable(
        &mut self,
        key: impl Into<Box<str>>,
        value: impl Into<Box<str>>,
    ) -> &mut Self {
        let key = key.into();

        // We will be adding more in the future
        #[allow(clippy::collapsible_if)]
        if key.is_ascii() {
            // We dynamically generate the following env flags.
            // Throw an error if the user tries to override to avoid confusion.
            if key.eq_ignore_ascii_case("RUSTFLAGS") {
                self.error = Some(anyhow::Error::msg("cannot set the `RUSTFLAGS` environment variable as it is dynamically generated"));
                return self;
            }
        }

        self.environment_variables.insert(key, value.into());
        self
    }

    /// Set the linker
    pub fn linker(&mut self, linker: impl Into<Box<str>>) -> &mut Self {
        self.linker = Some(linker.into());
        self
    }

    /// Set the cargo build wrapper
    pub fn cargo_build_wrapper(&mut self, cargo_build_wrapper: Vec<Box<str>>) -> &mut Self {
        if cargo_build_wrapper.is_empty() {
            self.error = Some(anyhow::Error::msg(
                "the cargo build wrapper cannot be empty",
            ));
            return self;
        }

        self.cargo_build_wrapper = Some(cargo_build_wrapper.into());
        self
    }

    /// Build a command to execute which will perform the cross compile
    pub fn build_command(&mut self) -> anyhow::Result<Command> {
        // Take all data from self, leaving it empty
        let target = self.target.take();
        let features = self.features.take();
        let profile = self.profile.take();
        let very_verbose = self.very_verbose;
        let environment_variables = std::mem::take(&mut self.environment_variables);
        let linker = self.linker.take();
        let cargo_build_wrapper = self.cargo_build_wrapper.take();
        let error = self.error.take();

        // Return error if the builder errored out somewhere
        if let Some(error) = error {
            return Err(error);
        }

        // Return error if missing a mandatory field
        let target = target.context("missing `target` field")?;
        let linker = linker.context("missing `linker` field")?;

        // Generate RUSTFLAGS environment variable
        let mut rust_flags = String::with_capacity(64);
        rust_flags.push_str("-Clinker=");
        rust_flags.push_str(&linker);
        rust_flags.push(' ');

        // Init cargo build command
        let mut command = if let Some(cargo_build_wrapper) = cargo_build_wrapper {
            let mut command = Command::new(&*cargo_build_wrapper[0]);
            if let Some(rest) = cargo_build_wrapper.get(1..) {
                command.args(rest.iter().map(|arg| &**arg));
            }
            command
        } else {
            let mut command = Command::new("cargo");
            command.arg("build");
            command
        };

        // Set target
        command.args(["--target", &target]);

        // Set features if present
        if let Some(features) = features {
            command.args(["--features", &features]);
        }

        // Set profile flag if requested
        if let Some(profile) = profile {
            command.arg(format!("--profile={profile}"));
        }

        // Set the very verbose flag if requested
        if very_verbose {
            command.arg("-vv");
        }

        // Add user environment variables
        command.envs(
            environment_variables
                .iter()
                .map(|(key, value)| (&**key, &**value)),
        );

        // Add RUSTFLAGS to command environment
        command.env("RUSTFLAGS", rust_flags);

        Ok(command)
    }
}

impl Default for CrossCompileCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
