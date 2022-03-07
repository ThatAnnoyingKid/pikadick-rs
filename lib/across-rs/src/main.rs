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

    // Setup command builder
    let mut command_builder = CrossCompileCommandBuilder::new();
    command_builder
        .target(options.target.as_str())
        .release(options.release)
        .very_verbose(options.very_verbose)
        .use_strip(options.use_strip);

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
                "`CMAKE_TOOLCHAIN_FILE` is already specified in the environment with value `{}`",
                value
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

    /// Whether to build in release mode
    pub release: bool,

    /// Whether to set the very verbose flag
    pub very_verbose: bool,

    /// Environment Variables
    pub environment_variables: HashMap<Box<str>, Box<str>>,

    /// The linker
    pub linker: Option<Box<str>>,

    /// Whether to strip the final binary
    pub use_strip: bool,

    /// This is some if the builder errored.
    pub error: Option<anyhow::Error>,
}

impl CrossCompileCommandBuilder {
    /// A cross compile command
    pub fn new() -> Self {
        Self {
            target: None,
            features: None,
            release: true,
            very_verbose: false,
            environment_variables: HashMap::with_capacity(16),
            linker: None,
            use_strip: false,

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

    /// Set the release flag
    pub fn release(&mut self, release: bool) -> &mut Self {
        self.release = release;
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

    /// Choose whether to strip the final binary
    pub fn use_strip(&mut self, use_strip: bool) -> &mut Self {
        self.use_strip = use_strip;
        self
    }

    /// Build a command to execute which will perform the cross compile
    pub fn build_command(&mut self) -> anyhow::Result<Command> {
        // Take all data from self, leaving it empty
        let target = self.target.take();
        let features = self.features.take();
        let release = self.release;
        let very_verbose = self.very_verbose;
        let environment_variables = std::mem::take(&mut self.environment_variables);
        let linker = self.linker.take();
        let use_strip = self.use_strip;
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
        rust_flags.push_str(&*linker);
        rust_flags.push(' ');
        if use_strip {
            // TODO: allow user to specify strip level
            rust_flags.push_str("-Cstrip=symbols");
            rust_flags.push(' ');
        }

        // Init cargo build command
        let mut command = Command::new("cargo");
        command.arg("build");

        // Set target
        command.args(&["--target", &*target]);

        // Set features if present
        if let Some(features) = features {
            command.args(&["--features", &*features]);
        }

        // Set release flag if requested
        if release {
            command.arg("--release");
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
