use camino::Utf8PathBuf;

/// CLI Options
#[derive(Debug, argh::FromArgs)]
#[argh(description = "The pikadick discord bot")]
pub struct CliOptions {
    #[argh(
        option,
        description = "the path to the config",
        default = "Utf8PathBuf::from(\"./config.toml\")"
    )]
    pub config: Utf8PathBuf,
}
