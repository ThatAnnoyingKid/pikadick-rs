use crate::{
    config::{
        CargoTomlConfig,
        FileConfig,
    },
    get_rustup_active_toolchain,
    get_rustup_installed_targets,
    util::make_deb_arch,
};
use anyhow::{
    ensure,
    Context as _,
};
use camino::{
    Utf8Path,
    Utf8PathBuf,
};

/// The Cli Context
pub struct Context {
    pub cargo_toml_config: CargoTomlConfig,
    pub file_config: FileConfig,

    pub rustup_toolchain: String,
    pub rustup_installed_targets: Vec<String>,

    metadata: cargo_metadata::Metadata,
    root_package: cargo_metadata::Package,

    cross_config: Option<Utf8PathBuf>,
}

impl Context {
    /// Create the context.
    pub fn new() -> anyhow::Result<Self> {
        ssh2::init();

        println!("Fetching cargo metadata...");
        let metadata = cargo_metadata::MetadataCommand::new()
            .exec()
            .context("failed to get cargo metadata")?;

        let root_package = metadata
            .root_package()
            .context("failed to locate root package")?
            .clone();

        println!("Loading config...");
        let cargo_toml_config = CargoTomlConfig::load_from_package(&root_package)?;
        let file_config = FileConfig::new()?;

        let rustup_toolchain =
            get_rustup_active_toolchain().context("failed to get the active toolchain")?;
        let rustup_installed_targets = get_rustup_installed_targets(rustup_toolchain.as_str())
            .with_context(|| {
                format!(
                    "failed to get rustup toolchains for toolchain `{}`",
                    rustup_toolchain
                )
            })?;
        println!("Current Toolchain: {}", rustup_toolchain);
        println!("Installed Targets: {:#?}", rustup_installed_targets);

        println!();

        Ok(Self {
            cargo_toml_config,
            file_config,

            rustup_toolchain,
            rustup_installed_targets,

            metadata,
            root_package,

            cross_config: None,
        })
    }

    /// Get the cargo deb package path
    pub fn get_cargo_deb_package_path(&self, target: &str) -> anyhow::Result<Utf8PathBuf> {
        let mut path = self.metadata.target_directory.clone();
        let name = self.get_cargo_deb_package_name(target)?;

        path.extend([target, "debian", &name]);

        Ok(path)
    }

    /// Get the cargo deb package name
    pub fn get_cargo_deb_package_name(&self, target: &str) -> anyhow::Result<String> {
        let deb_arch = make_deb_arch(target)?;

        Ok(format!(
            "{}_{}_{}.deb",
            self.root_package.name, self.root_package.version, deb_arch
        ))
    }

    /// Set the cross config path
    pub fn set_cross_config<P>(&mut self, path: P)
    where
        P: AsRef<Utf8Path>,
    {
        self.cross_config = Some(path.as_ref().into());
    }

    /// Package a target
    pub fn package_target(&self, target: &str) -> anyhow::Result<()> {
        println!("Packaging for `{}`...", target);
        println!();

        ensure!(
            self.rustup_target_is_installed(target),
            "rustup target `{}` is not installed",
            target
        );

        // Building
        {
            println!("Building...");
            let mut command = std::process::Command::new("across");
            command.args(["--target", target]).arg("--release");
            if let Some(cross_config) = self.cross_config.as_ref() {
                command.arg("--config").arg(cross_config);
            }
            let status = command.status().context("failed to spawn build command")?;
            ensure!(status.success(), "build command failed");
            println!();
        }

        // Packaging
        {
            println!("Packaging...");
            let status = std::process::Command::new("cargo")
                .arg("deb")
                .args(["--target", target])
                .arg("--no-build")
                .arg("--no-strip")
                .arg("-v")
                .status()
                .context("failed to spawn package command")?;
            ensure!(status.success(), "build command failed");
            println!();
        }

        println!("Done packaging `{}`", target);
        println!();

        Ok(())
    }

    /// Returns true if a given rustup target is installed
    pub fn rustup_target_is_installed(&self, target: &str) -> bool {
        self.rustup_installed_targets
            .iter()
            .any(|installed_target| installed_target == target)
    }
}
