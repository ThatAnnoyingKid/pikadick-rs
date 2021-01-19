use parking_lot::Mutex as PMutex;
use slog::{
    info,
    Drain,
    Logger,
};
use std::{
    fs::File,
    io::Write,
    sync::Arc,
};

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

/// Setup a logger
pub fn setup() -> (Logger, DelayWriter) {
    let term_drain = {
        let decorator = slog_term::TermDecorator::new().build();
        slog_term::FullFormat::new(decorator).build()
    };

    let file_writer = DelayWriter::new();

    let file_drain = {
        let decorator = slog_term::PlainDecorator::new(file_writer.clone());
        slog_term::FullFormat::new(decorator).build()
    };

    let drain = slog_async::Async::new(slog::Duplicate(term_drain, file_drain).fuse())
        .build()
        .fuse();

    let log = slog::Logger::root(drain, slog::o!());

    info!(log, "Initalized Logger");

    (log, file_writer)
}
