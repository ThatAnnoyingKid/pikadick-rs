use anyhow::{
    ensure,
    Context,
};
use cargo_metadata::MetadataCommand;
use flate2::{
    write::GzEncoder,
    Compression,
};
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
};

#[derive(Debug, argh::FromArgs)]
#[argh(description = "a tool to simplify deploying to Raspberry Pis")]
pub struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
pub enum Subcommand {
    Package(PackageOptions),
}

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "package",
    description = "a command to package an application"
)]
pub struct PackageOptions {
    #[argh(option, description = "the target")]
    target: Option<String>,

    #[argh(switch, description = "use the release profile")]
    release: bool,

    #[argh(option, long = "exe-name", description = "the exe name")]
    exe_name: String,
}

fn main() {
    let options = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };

    std::process::exit(code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    match options.subcommand {
        Subcommand::Package(options) => {
            println!("Fetching cargo metadata...");
            println!();
            let metadata = MetadataCommand::new()
                .exec()
                .context("failed to get cargo metadata")?;

            let target_str = options.target.as_deref().unwrap_or("default");
            let profile_str = if options.release { "release" } else { "debug" };

            // Paths setup
            let base_path = {
                let mut path = PathBuf::from(&metadata.target_directory);
                if let Some(target_str) = options.target.as_deref() {
                    path.push(target_str);
                }
                path.push(profile_str);
                path
            };
            let exe_path = base_path.join(options.exe_name.as_str());
            let package_path = {
                let mut path = base_path.join(options.exe_name.as_str());
                path.set_extension("tar.gz");

                path
            };

            println!("Packaging Options: ");
            println!("    Target: {}", target_str);
            println!("    Profile: {}", profile_str);
            println!("    Exe Name: {}", options.exe_name);
            println!();

            println!("    Base Path: {}", base_path.display());
            println!("    Exe Path: {}", exe_path.display());
            println!("    Package Path: {}", package_path.display());
            println!();

            ensure!(exe_path.exists(), "the exe file does not exist");

            // Overwrite old package
            println!("Creating package...");
            let mut package_file =
                File::create(&package_path).context("failed to open package file")?;
            {
                let mut package_compressor = GzEncoder::new(&mut package_file, Compression::best());
                {
                    let mut package_tar = tar::Builder::new(&mut package_compressor);
                    println!("Adding exe...");
                    package_tar
                        .append_path_with_name(exe_path, options.exe_name)
                        .context("failed to add exe to tar")?;
                    package_tar
                        .finish()
                        .context("failed to finish creating tar")?;
                }
                package_compressor
                    .finish()
                    .context("failed to finish compressing")?;
            }
            package_file
                .flush()
                .context("failed to flush package file")?;
            package_file
                .sync_all()
                .context("failed to sync package file")?;
            println!();
        }
    }
    Ok(())
}
