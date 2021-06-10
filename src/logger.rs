use crate::config::LogConfig;
use anyhow::Context;
use parking_lot::Mutex;
use std::{
    io::Write,
    sync::Arc,
};
use tonic::metadata::{
    MetadataKey,
    MetadataMap,
};
use tracing_subscriber::layer::SubscriberExt;

/// The mut impl of a DelayWriter
#[derive(Debug)]
pub enum DelayWriterInner<W> {
    /// The buffered data.
    Buffer(Vec<u8>),

    /// The file being written to.
    Writer(W),
}

impl<W> DelayWriterInner<W> {
    /// Make a new DelayWriterInner with an empty buffer
    fn new() -> Self {
        Self::Buffer(Vec::with_capacity(1_000_000))
    }
}

impl<W> DelayWriterInner<W>
where
    W: Write,
{
    /// Try to init this DelayWriterInner with a file.
    ///
    /// # Error
    /// Will return an error if this is already initalized.
    fn init(&mut self, mut writer: W) -> Result<(), std::io::Error> {
        let buffer = match self {
            Self::Buffer(bytes) => bytes,
            Self::Writer(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "already initalized",
                ));
            }
        };

        writer.write_all(buffer)?;

        *self = Self::Writer(writer);

        Ok(())
    }
}

impl<W> Write for DelayWriterInner<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Buffer(buffer) => buffer.write(buf),
            Self::Writer(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Buffer(buffer) => buffer.flush(),
            Self::Writer(writer) => writer.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Buffer(buffer) => buffer.write_all(buf),
            Self::Writer(writer) => writer.write_all(buf),
        }
    }
}

impl<W> DelayWriter<W> {
    /// Create a new DelayWriter
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(DelayWriterInner::new())))
    }
}

/// A writer that buffers data until it is assigned a file to write to.
#[derive(Debug)]
pub struct DelayWriter<W>(Arc<Mutex<DelayWriterInner<W>>>);

impl<W> DelayWriter<W>
where
    W: Write,
{
    /// Try to init this DelayWriter
    pub fn init(&self, writer: W) -> Result<(), std::io::Error> {
        let mut lock = self.0.lock();
        lock.init(writer)
    }
}

impl<W> Default for DelayWriter<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W> Write for DelayWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.0.lock();

        lock.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut lock = self.0.lock();

        lock.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut lock = self.0.lock();

        lock.write_all(buf)
    }
}

impl<W> Clone for DelayWriter<W> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Try to setup a logger
pub fn setup(
    config: Option<&LogConfig>,
) -> anyhow::Result<(
    DelayWriter<tracing_appender::rolling::RollingFileAppender>,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let file_writer = DelayWriter::new();
    let (nonblocking_file_writer, guard) = tracing_appender::non_blocking(file_writer.clone());

    let env_filter = tracing_subscriber::filter::EnvFilter::default()
        .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into());
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
