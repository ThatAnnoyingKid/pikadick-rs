mod delay_writer;

pub use self::delay_writer::DelayWriter;
use crate::config::Config;
use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_log::LogTracer;
use tracing_subscriber::{
    filter::EnvFilter,
    layer::SubscriberExt,
};

/// Try to setup a logger.
///
/// Must be called from a tokio runtime.
pub fn setup(config: &Config) -> anyhow::Result<WorkerGuard> {
    let file_writer = tracing_appender::rolling::hourly(config.log_file_dir(), "log.txt");
    let (nonblocking_file_writer, guard) = tracing_appender::non_blocking(file_writer);

    let mut env_filter = EnvFilter::default();
    // If the user provides logging directives, use them
    for directive in config.log.directives.iter() {
        env_filter = env_filter.add_directive(
            directive
                .parse()
                .context("failed to parse logging directive")?,
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

    tracing::subscriber::set_global_default(subscriber).context("failed to set subscriber")?;

    LogTracer::init().context("failed to init log tracer")?;

    Ok(guard)
}
