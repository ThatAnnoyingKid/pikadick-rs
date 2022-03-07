use anyhow::{
    bail,
    ensure,
    Context,
};

/// Get the active toolchain
pub fn get_rustup_active_toolchain() -> anyhow::Result<String> {
    let output = std::process::Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .context("failed to spawn rustup")?;
    ensure!(output.status.success(), "rustup exit code was non-zero");

    let stdout = std::str::from_utf8(&output.stdout).context("output was not utf8")?;
    let mut iter = stdout.split(' ');

    let toolchain = iter.next().context("missing toolchain")?;

    Ok(toolchain.to_string())
}

/// Get targets installed by rustup
pub fn get_rustup_installed_targets<'a>(
    toolchain: impl Into<Option<&'a str>>,
) -> anyhow::Result<Vec<String>> {
    let toolchain = toolchain.into();

    let mut cmd = std::process::Command::new("rustup");
    cmd.args(["target", "list", "--installed"]);

    if let Some(toolchain) = toolchain {
        cmd.args(["--toolchain", toolchain]);
    }

    let output = cmd.output().context("failed to spawn rustup")?;

    ensure!(output.status.success(), "rustup exit code was non-zero");

    let stdout = std::str::from_utf8(&output.stdout).context("output was not utf8")?;
    Ok(stdout
        .split('\n')
        .map(|target_str| target_str.trim())
        .filter(|target_str| !target_str.is_empty())
        .map(|target_str| target_str.to_string())
        .collect())
}

/// Make a target triple into a deb arch
pub fn make_deb_arch(target: &str) -> anyhow::Result<&'static str> {
    let mut iter = target.split('-');
    let arch = iter.next().context("missing arch string")?;
    let vendor = iter.next().context("missing vendor string")?;
    let os_type = iter.next().context("missing os type")?;
    let environment_type = iter.next();

    match (arch, vendor, os_type, environment_type) {
        ("aarch64", _, _, _) => Ok("arm64"),
        (arm, _, _, Some(gnueabi)) if arm.starts_with("arm") && gnueabi.ends_with("hf") => {
            Ok("armhf")
        }
        _ => bail!("failed to convert target triple `{}` to a deb arch", target),
    }
}
