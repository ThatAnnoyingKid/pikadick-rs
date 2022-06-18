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
        let offset = match time::UtcOffset::current_local_offset() {
            Ok(offset) => offset,
            Err(error) => {
                println!("failed to get local offset ({error}), using UTC...");
                time::UtcOffset::UTC
            }
        };

        let start = Instant::now();
        let boot_time = CacheContext::new()
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
        let start = Instant::now();
        let uptime = CacheContext::new()
            .get_uptime()
            .expect("failed to get uptime");
        let elapsed = start.elapsed();

        println!("Uptime: {:?}\nTime: {:?}", uptime, elapsed);
    }

    #[test]
    fn hostname() {
        let start = Instant::now();
        let hostname = CacheContext::new()
            .get_hostname()
            .expect("failed to get hostname");
        let elapsed = start.elapsed();

        println!("Hostname: {}\nTime: {:?}", hostname, elapsed);
    }

    #[test]
    fn architecture() {
        let start = Instant::now();
        let arch = CacheContext::new()
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
        let start = Instant::now();
        let system_name = CacheContext::new()
            .get_system_name()
            .expect("failed to get system name");
        let elapsed = start.elapsed();

        println!("System Name: {:?}\nTime: {:?}", system_name, elapsed);
    }

    #[test]
    fn system_version() {
        let start = Instant::now();
        let system_version = CacheContext::new()
            .get_system_version()
            .expect("failed to get system version");
        let elapsed = start.elapsed();

        println!("System Version: {:?}\nTime: {:?}", system_version, elapsed);
    }

    #[test]
    fn total_memory() {
        let start = Instant::now();
        let total_memory = CacheContext::new()
            .get_total_memory()
            .expect("failed to get total memory");
        let elapsed = start.elapsed();

        println!("Total Memory: {}\nTime: {:?}", total_memory, elapsed);
    }

    #[test]
    fn available_memory() {
        let start = Instant::now();
        let available_memory = CacheContext::new()
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
        let start = Instant::now();
        let total_swap = CacheContext::new()
            .get_total_swap()
            .expect("failed to get total swap");
        let elapsed = start.elapsed();

        println!("Total Swap: {}\nTime: {:?}", total_swap, elapsed);
    }

    #[test]
    fn available_swap() {
        let start = Instant::now();
        let available_swap = CacheContext::new()
            .get_available_swap()
            .expect("failed to get available swap memory");
        let elapsed = start.elapsed();

        println!("Available Swap: {}\nTime: {:?}", available_swap, elapsed);
    }
}
