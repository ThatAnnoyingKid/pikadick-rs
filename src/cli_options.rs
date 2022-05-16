use std::path::PathBuf;

/// CLI Options
#[derive(Debug, argh::FromArgs)]
#[argh(description = "The pikadick discord bot")]
pub struct CliOptions {
    #[argh(
        option,
        description = "the path to the config",
        default = "PathBuf::from(\"./config.toml\")"
    )]
    pub config: PathBuf,
}
