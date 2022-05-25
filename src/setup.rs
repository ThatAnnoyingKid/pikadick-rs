use crate::config::{
    Config,
    Severity,
};
use anyhow::{
    ensure,
    Context,
};
use camino::Utf8Path;

/// Load a config.
///
/// This prints to the stderr directly.
/// It is intended to be called BEFORE the loggers are set up.
pub(crate) fn load_config(path: &Utf8Path) -> anyhow::Result<Config> {
    eprintln!("loading `{}`...", path);
    let mut config =
        Config::load_from_path(path).with_context(|| format!("failed to load `{}`", path))?;

    eprintln!("validating config...");
    let errors = config.validate();
    let mut error_count = 0;
    for e in errors {
        match e.severity() {
            Severity::Warn => {
                eprintln!("validation warning: {}", e.error());
            }
            Severity::Error => {
                eprintln!("validation error: {}", e.error());
                error_count += 1;
            }
        }
    }

    ensure!(
        error_count == 0,
        "validation failed with {error_count} errors."
    );

    Ok(config)
}
