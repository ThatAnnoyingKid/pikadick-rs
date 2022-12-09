mod config;
mod context;
mod util;

use self::{
    context::Context,
    util::{
        get_rustup_active_toolchain,
        get_rustup_installed_targets,
    },
};
use anyhow::{
    ensure,
    Context as _,
};
pub use camino;
use rand::distributions::DistString;
use std::{
    fs::File,
    io::Write,
    net::TcpStream,
};

#[derive(argh::FromArgs)]
#[argh(description = "a command to help deploy to raspberry pis")]
pub struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
pub enum Subcommand {
    Package(PackageOptions),
    Deploy(DeployOptions),
}

#[derive(argh::FromArgs)]
#[argh(subcommand, description = "package a build", name = "package")]
pub struct PackageOptions {}

#[derive(argh::FromArgs)]
#[argh(subcommand, description = "deploy a package", name = "deploy")]
pub struct DeployOptions {
    #[argh(
        option,
        long = "name",
        short = 'n',
        description = "the name of the target machine in the deploy config"
    )]
    pub name: String,
}

fn main() {
    let options: Options = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            1
        }
    };

    std::process::exit(code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let ctx = Context::new()?;

    match options.subcommand {
        Subcommand::Package(_options) => {
            ensure!(!ctx.cargo_toml_config.targets.is_empty(), "missing targets");

            println!("Packaging...");
            println!();
            for target in ctx.cargo_toml_config.targets.iter() {
                ctx.package_target(target)?;
            }
        }
        Subcommand::Deploy(options) => {
            let machine_config = ctx
                .file_config
                .get_machine_config(&options.name)
                .with_context(|| {
                    format!("missing machine config for machine `{}`", options.name)
                })?;

            println!("Packaging...");
            ctx.package_target(&machine_config.target)?;

            println!("Deploying to `{}`...", options.name);

            let deb_package_path =
                ctx.get_cargo_deb_package_path(machine_config.target.as_str())?;

            println!(
                "Connecting to `ssh://{}@{}`...",
                machine_config.username, machine_config.address
            );
            let tcp_stream = TcpStream::connect(&machine_config.address)
                .context("failed to connect to ssh server")?;
            let mut session = ssh2::Session::new().context("failed to create ssh session")?;
            session.set_tcp_stream(tcp_stream);

            println!("Sending handshake...");
            session.handshake().context("ssh handshake failed")?;

            println!("Logging in...");
            session
                .userauth_password(&machine_config.username, &machine_config.password)
                .context("failed to log in")?;
            ensure!(session.authenticated(), "failed to log in");
            println!();

            println!("Opening SFTP channel...");
            let sftp = session.sftp().context("failed to open sftp channel")?;

            println!("Copying package...");
            let local_package_file_path = deb_package_path;
            let local_package_file = File::open(&local_package_file_path)?;
            let local_package_file_metadata = local_package_file.metadata()?;

            let file_name = {
                let mut file_stem = local_package_file_path
                    .file_stem()
                    .context("missing file stem")?
                    .to_owned();
                let file_extension = local_package_file_path
                    .extension()
                    .context("missing file extension")?;

                // Push RNG string to randomize tmp file
                let mut file_stem_extension = String::from("-");
                rand::distributions::Alphanumeric.append_string(
                    &mut rand::thread_rng(),
                    &mut file_stem_extension,
                    10,
                );
                file_stem_extension.push_str("-tmp");
                file_stem.push_str(&file_stem_extension);

                // Push extension
                file_stem.push('.');
                file_stem.push_str(file_extension);

                file_stem
            };

            // TODO: Don't assume /tmp is the temp dir
            let remote_package_file_path = format!("/tmp/{}", file_name);
            let mut remote_package_file = sftp.open_mode(
                remote_package_file_path.as_ref(),
                ssh2::OpenFlags::WRITE | ssh2::OpenFlags::TRUNCATE,
                0o600, // Prevent users from tampering with the file.
                ssh2::OpenType::File,
            )?;

            // Perform copy
            let metadata_len = local_package_file_metadata.len();
            let progress_bar = indicatif::ProgressBar::new(metadata_len);
            let progress_bar_style_template = "[Time = {elapsed_precise} | ETA = {eta_precise} | Speed = {bytes_per_sec}] {wide_bar} {bytes}/{total_bytes}";
            let progress_bar_style = indicatif::ProgressStyle::default_bar()
                .template(progress_bar_style_template)
                .expect("invalid progress bar style template");
            progress_bar.set_style(progress_bar_style);
            // remote_package_file.set_len(metadata_len)?;
            let bytes_copied = std::io::copy(
                &mut progress_bar.wrap_read(local_package_file),
                &mut remote_package_file,
            )?;
            progress_bar.finish();
            ensure!(
                metadata_len == bytes_copied,
                "file length changed during transfer, (expected) {} != (actual) {}",
                metadata_len,
                bytes_copied
            );
            remote_package_file.flush()?;
            remote_package_file.fsync()?;

            println!("Installing...");
            println!();
            {
                let mut ssh_channel = session.channel_session()?;
                ssh_channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
                ssh_channel.exec(
                    format!("DEBIAN_FRONTEND=noninteractive sudo apt-get -y --fix-broken reinstall -o DPkg::options::=\"--force-confdef\" -o DPkg::options::=\"--force-confold\" {}", remote_package_file_path).as_str(),
                )?;

                {
                    let mut stderr_lock = std::io::stderr();
                    std::io::copy(&mut ssh_channel, &mut stderr_lock)?;
                }

                ssh_channel.close()?;
                ssh_channel.wait_close()?;

                let exit_status = ssh_channel.exit_status()?;

                ensure!(
                    exit_status == 0,
                    "command exited with exit code {}",
                    exit_status
                );
            }

            println!("Deleting tmp file...");
            println!();
            {
                let mut ssh_channel = session.channel_session()?;
                ssh_channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
                ssh_channel.exec(format!("rm {}", remote_package_file_path).as_str())?;

                {
                    let mut stderr_lock = std::io::stderr();
                    std::io::copy(&mut ssh_channel, &mut stderr_lock)?;
                }

                ssh_channel.close()?;
                ssh_channel.wait_close()?;

                let exit_status = ssh_channel.exit_status()?;

                ensure!(
                    exit_status == 0,
                    "command exited with exit code {}",
                    exit_status
                );
            }
            println!();
        }
    }

    Ok(())
}
