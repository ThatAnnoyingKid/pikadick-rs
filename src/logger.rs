mod delay_writer;

pub use self::delay_writer::DelayWriter;
use crate::config::Config;
use anyhow::Context;
use opentelemetry_otlp::WithExportConfig;
use tonic::metadata::{
    MetadataKey,
    MetadataMap,
};
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
    let file_writer = tracing_appender::rolling::hourly(&config.data_dir, "log.txt");
    let (nonblocking_file_writer, guard) = tracing_appender::non_blocking(file_writer);

    // Only enable pikadick since serenity likes puking in the logs during connection failures
    // serenity's framework section seems ok as well
    let env_filter = EnvFilter::default()
        .add_directive(
            "pikadick=info"
                .parse()
                .context("failed to parse logging directive")?,
        )
        .add_directive(
            "serenity::framework::standard=info"
                .parse()
                .context("failed to parse logging directive")?,
        );
    let stderr_formatting_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let file_formatting_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(nonblocking_file_writer);

    let opentelemetry_layer = if let Some(config) = config.log.as_ref() {
        opentelemetry::global::set_error_handler(|e| {
            // Print to stderr.
            // There was an error logging something, so we avoid using the logging system.
            eprintln!("opentelemetry error: {:?}", anyhow::anyhow!(e));
        })
        .context("failed to set opentelemetry error handler")?;

        let mut map = MetadataMap::with_capacity(config.headers.len());
        for (k, v) in config.headers.iter() {
            let k = MetadataKey::from_bytes(k.as_bytes()).context("invalid header name")?;
            map.insert(k, v.parse().context("invalid header value")?);
        }

        let exporter = {
            let mut exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_metadata(map)
                .with_tls_config(Default::default());

            if let Some(endpoint) = config.endpoint.as_ref() {
                exporter = exporter.with_endpoint(endpoint);
            }

            exporter
        };

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .install_batch(opentelemetry::runtime::Tokio)
            .context("failed to install otlp opentelemetry exporter")?;

        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(file_formatting_layer)
        .with(stderr_formatting_layer);

    if let Some(opentelemetry_layer) = opentelemetry_layer {
        let subscriber = subscriber.with(opentelemetry_layer);

        tracing::subscriber::set_global_default(subscriber).context("failed to set subscriber")?;
    } else {
        tracing::subscriber::set_global_default(subscriber).context("failed to set subscriber")?;
    }

    LogTracer::init().context("failed to init log tracer")?;

    Ok(guard)
}
