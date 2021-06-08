use anyhow::Context;
use parking_lot::Mutex as PMutex;
use std::{
    fs::File,
    io::Write,
    sync::Arc,
};
use tracing_subscriber::layer::SubscriberExt;

/// The mut impl of a DelayWriter
#[derive(Debug)]
pub enum DelayWriterInner {
    /// The buffered data.
    Buffer(Vec<u8>),

    /// The file being written to.
    File(File),
}

impl DelayWriterInner {
    /// Make a new DelayWriterInner with an empty buffer
    fn new() -> Self {
        Self::Buffer(Vec::new())
    }

    /// Try to init this DelayWriterInner with a file. Will return an error if this is already initalized.
    fn init(&mut self, mut file: File) -> Result<(), std::io::Error> {
        let buffer = match self {
            Self::Buffer(bytes) => bytes,
            Self::File(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Already Initalized",
                ));
            }
        };

        file.write_all(buffer)?;

        *self = Self::File(file);

        Ok(())
    }
}

impl Write for DelayWriterInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Buffer(buffer) => buffer.write(buf),
            Self::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Buffer(buffer) => buffer.flush(),
            Self::File(file) => file.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Buffer(buffer) => buffer.write_all(buf),
            Self::File(file) => file.write_all(buf),
        }
    }
}

/// A writer that buffers data until it is assigned a file to write to.
#[derive(Clone, Debug)]
pub struct DelayWriter(Arc<PMutex<DelayWriterInner>>);

impl DelayWriter {
    /// Create a new DelayWriter
    pub fn new() -> Self {
        Self(Arc::new(PMutex::new(DelayWriterInner::new())))
    }

    /// Try to init this DelayWriter
    pub fn init(&self, file: File) -> Result<(), std::io::Error> {
        let mut lock = self.0.lock();
        lock.init(file)
    }
}

impl Default for DelayWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for DelayWriter {
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

/// Try to setup a logger
pub fn setup() -> anyhow::Result<DelayWriter> {
    let file_writer = DelayWriter::new();
    let file_writer_clone = file_writer.clone();

    let env_filter = tracing_subscriber::filter::EnvFilter::default()
        .add_directive(tracing_subscriber::filter::LevelFilter::INFO.into());
    let stderr_formatting_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let file_formatting_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(move || file_writer_clone.clone());

    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(file_formatting_layer)
        .with(stderr_formatting_layer);

    tracing::subscriber::set_global_default(subscriber).context("failed to set subscriber")?;

    /*
    let term_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors_line.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .level_for("tracing", log::LevelFilter::Warn)
        .level_for("serenity", log::LevelFilter::Warn)
        .level_for(
            "serenity::client::bridge::gateway::shard_runner",
            log::LevelFilter::Error,
        )
        .level_for("sqlx::query", log::LevelFilter::Error)
        .chain(std::io::stdout());
        */

    Ok(file_writer)
}
