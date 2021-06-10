mod delay_writer;

pub use self::delay_writer::DelayWriter;
use crate::config::LogConfig;
use anyhow::Context;
use tonic::metadata::{
    MetadataKey,
    MetadataMap,
};
use tracing_subscriber::layer::SubscriberExt;

/// Try to setup a logger
pub fn setup(
    config: Option<&LogConfig>,
) -> anyhow::Result<(
    DelayWriter<tracing_appender::rolling::RollingFileAppender>,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let file_writer = DelayWriter::new();
    let (nonblocking_file_writer, guard) = tracing_appender::non_blocking(file_writer.clone());

    // Only enable pikadick since serenity like puking in the logs during connection failures
    let env_filter = tracing_subscriber::filter::EnvFilter::default().add_directive(
        "pikadick=info"
            .parse()
            .context("failed to parse logging directive")?,
    );
    let stderr_formatting_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let file_formatting_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(nonblocking_file_writer);

    let opentelemetry_layer = if let Some(config) = config {
        let mut map = MetadataMap::with_capacity(config.headers.len());
        for (k, v) in config.headers.iter() {
            let k = MetadataKey::from_bytes(k.as_bytes()).context("invalid header name")?;
            map.insert(k, v.parse().context("invalid header value")?);
        }

        let tracer = {
            let mut tracer = opentelemetry_otlp::new_pipeline();

            if let Some(endpoint) = config.endpoint.as_ref() {
                tracer = tracer.with_endpoint(endpoint);
            }

            tracer
                .with_tonic()
                .with_metadata(map)
                .with_tls_config(Default::default())
                .install_batch(opentelemetry::runtime::Tokio)
                .context("failed to install otlp opentelemetry exporter")?
        };

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

    tracing_log::LogTracer::init().context("failed to init log tracer")?;

    Ok((file_writer, guard))
}
