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

#[derive(Debug)]
pub enum DelayWriterInner {
    Buffer(Vec<u8>),
    File(File),
}

impl DelayWriterInner {
    fn new() -> Self {
        DelayWriterInner::Buffer(Vec::new())
    }

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

#[derive(Clone, Debug)]
pub struct DelayWriter(Arc<PMutex<DelayWriterInner>>);

impl DelayWriter {
    pub fn new() -> Self {
        DelayWriter(Arc::new(PMutex::new(DelayWriterInner::new())))
    }

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
