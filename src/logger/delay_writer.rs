use parking_lot::Mutex;
use std::{
    io::Write,
    sync::Arc,
};

const DEFAULT_CAPACITY: usize = 1_000_000;

/// The mut impl of a DelayWriter
#[derive(Debug)]
pub(crate) enum DelayWriterInner<W> {
    /// The buffered data.
    Buffer(Vec<u8>),

    /// The file being written to.
    Writer(W),
}

impl<W> DelayWriterInner<W> {
    /// Make a new DelayWriterInner with an empty buffer
    fn new() -> Self {
        Self::Buffer(Vec::with_capacity(DEFAULT_CAPACITY))
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
