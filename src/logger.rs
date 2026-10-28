mod delay_writer;

pub use self::delay_writer::DelayWriter;
use crate::config::Config;
use anyhow::Context;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{
        RollingFileAppender,
        Rotation,
    },
};
use tracing_log::LogTracer;
use tracing_subscriber::{
    filter::EnvFilter,
    layer::SubscriberExt,
};

const LOG_PREFIX: &str = "log";
const LOG_SUFFIX: &str = "txt";

/// Try to setup a logger.
///
/// Must be called from a tokio runtime.
pub fn setup(config: &Config) -> anyhow::Result<WorkerGuard> {
    let file_writer = RollingFileAppender::builder()
        .rotation(Rotation::HOURLY)
        .filename_prefix(LOG_PREFIX)
        .filename_suffix(LOG_SUFFIX)
        .build(config.log_file_dir())?;
    let (nonblocking_file_writer, guard) = tracing_appender::non_blocking(file_writer);

    let mut env_filter = EnvFilter::default();
    // If the user provides logging directives, use them
    for directive in config.log.directives.iter() {
        env_filter = env_filter.add_directive(
            directive
                .parse()
                .context("Failed to parse logging directive")?,
        );
    }

    let stderr_formatting_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let file_formatting_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(nonblocking_file_writer);

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(file_formatting_layer)
        .with(stderr_formatting_layer);

    tracing::subscriber::set_global_default(subscriber).context("Failed to set subscriber")?;

    LogTracer::init().context("Failed to init log tracer")?;

    Ok(guard)
}
