#![allow(clippy::uninlined_format_args)]

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        use self::windows as imp;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use self::linux as imp;
    } else {
        compile_error!("unsupported platform");
    }
}

pub use platforms::Arch;
use std::time::{
    Duration,
    SystemTime,
};

/// The library error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An io error occured
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Invalid UTF8 string
    #[error(transparent)]
    InvalidUtf8String(#[from] std::string::FromUtf8Error),

    /// Invalid UTF16 string
    #[error(transparent)]
    InvalidUtf16String(#[from] std::string::FromUtf16Error),

    /// Invalid Utf8 OsStr
    #[error("invalid utf8 os str")]
    InvalidUtf8OsStr,

    /// The value is missing for some reason
    #[error("missing value")]
    MissingValue,

    /// A generic error occured.
    #[error("{0}")]
    Generic(&'static str),
}

/// A context for caching data related to information queries.
///
/// A system may occasionally give more data than requested when asked.
/// Using this allows the use of that data without fetching it again.
/// A new context should be created if new data is required, as it will happily reuse old data if it has it.
#[derive(Debug)]
pub struct CacheContext {
    inner: imp::CacheContext,
}

impl CacheContext {
    /// Make a new [`CacheContext`].
    pub fn new() -> Self {
        Self {
            inner: imp::CacheContext::new(),
        }
    }

    /// Get the boot time.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_boot_time(&self) -> Result<SystemTime, Error> {
        self.inner.get_boot_time()
    }

    /// Get the computer uptime.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_uptime(&self) -> Result<Duration, Error> {
        self.inner.get_uptime()
    }

    /// Get the hostname.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_hostname(&self) -> Result<String, Error> {
        self.inner.get_hostname()
    }

    /// Get the arch.
    ///
    /// # Returns
    /// This returns `None` if the arch is unknown.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_architecture(&self) -> Result<Option<Arch>, Error> {
        self.inner.get_architecture()
    }

    /// Get the system name, or the name of the operating system.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_system_name(&self) -> Result<Option<String>, Error> {
        self.inner.get_system_name()
    }

    /// Get the operating system version.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_system_version(&self) -> Result<String, Error> {
        self.inner.get_system_version()
    }

    /// Get the total amount of memory in the computer, in bytes.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_total_memory(&self) -> Result<u64, Error> {
        self.inner.get_total_memory()
    }

    /// Get the available amount of memory in the computer, in bytes.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_available_memory(&self) -> Result<u64, Error> {
        self.inner.get_available_memory()
    }

    /// Get the total amount of swap in the computer, in bytes.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_total_swap(&self) -> Result<u64, Error> {
        self.inner.get_total_swap()
    }

    /// Get the available amount of swap in the computer, in bytes.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_available_swap(&self) -> Result<u64, Error> {
        self.inner.get_available_swap()
    }

    /// Get the number of logical cpu cores.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn count_logical_cpus(&self) -> Result<usize, Error> {
        self.inner.count_logical_cpus()
    }
}

impl Default for CacheContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn boot_time() {
        let cache_context = CacheContext::new();
        let offset = match time::UtcOffset::current_local_offset() {
            Ok(offset) => offset,
            Err(error) => {
                println!("failed to get local offset ({error}), using UTC...");
                time::UtcOffset::UTC
            }
        };

        let start = Instant::now();
        let boot_time = cache_context
            .get_boot_time()
            .expect("failed to get boot time");
        let elapsed = start.elapsed();

        println!(
            "Boot Time: {}\nTime: {:?}",
            time::OffsetDateTime::from(boot_time).to_offset(offset),
            elapsed
        );
    }

    #[test]
    fn uptime() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let uptime = cache_context.get_uptime().expect("failed to get uptime");
        let elapsed = start.elapsed();

        println!("Uptime: {:?}\nTime: {:?}", uptime, elapsed);
    }

    #[test]
    fn hostname() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let hostname = cache_context
            .get_hostname()
            .expect("failed to get hostname");
        let elapsed = start.elapsed();

        println!("Hostname: {}\nTime: {:?}", hostname, elapsed);
    }

    #[test]
    fn architecture() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let arch = cache_context
            .get_architecture()
            .expect("failed to get arch");
        let elapsed = start.elapsed();

        println!(
            "Arch: {}\nTime: {:?}",
            arch.map(|arch| arch.as_str()).unwrap_or("unknown"),
            elapsed
        );
    }

    #[test]
    fn system_name() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let system_name = cache_context
            .get_system_name()
            .expect("failed to get system name");
        let elapsed = start.elapsed();

        println!("System Name: {:?}\nTime: {:?}", system_name, elapsed);
    }

    #[test]
    fn system_version() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let system_version = cache_context
            .get_system_version()
            .expect("failed to get system version");
        let elapsed = start.elapsed();

        println!("System Version: {:?}\nTime: {:?}", system_version, elapsed);
    }

    #[test]
    fn total_memory() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let total_memory = cache_context
            .get_total_memory()
            .expect("failed to get total memory");
        let elapsed = start.elapsed();

        println!("Total Memory: {}\nTime: {:?}", total_memory, elapsed);
    }

    #[test]
    fn available_memory() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let available_memory = cache_context
            .get_available_memory()
            .expect("failed to get available memory");
        let elapsed = start.elapsed();

        println!(
            "Available Memory: {}\nTime: {:?}",
            available_memory, elapsed
        );
    }

    #[test]
    fn total_swap() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let total_swap = cache_context
            .get_total_swap()
            .expect("failed to get total swap");
        let elapsed = start.elapsed();

        println!("Total Swap: {}\nTime: {:?}", total_swap, elapsed);
    }

    // TODO: I think swap calc is just broken now.
    /*
    #[test]
    fn available_swap() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let available_swap = cache_context
            .get_available_swap()
            .expect("failed to get available swap memory");
        let elapsed = start.elapsed();

        println!("Available Swap: {}\nTime: {:?}", available_swap, elapsed);
    }
    */

    #[test]
    fn logical_cpus() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let logical_cpus = cache_context
            .count_logical_cpus()
            .expect("failed to get count logical cpus");
        let elapsed = start.elapsed();

        println!("Logical Cpus: {}\nTime: {:?}", logical_cpus, elapsed);
    }
}
