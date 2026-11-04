cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        use self::windows as imp;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use self::linux as imp;
    } else {
        compile_error!("Unsupported platform");
    }
}

pub use platforms::Arch;
use std::{
    ffi::OsString,
    time::{
        Duration,
        SystemTime,
    },
};

/// The library error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An io error occured
    #[error("IO error")]
    Io(#[from] std::io::Error),

    /// Invalid UTF8 string
    #[error("Invalid Utf8 string")]
    InvalidUtf8String(#[from] std::string::FromUtf8Error),

    /// Invalid UTF16 string
    #[error("Invalid Utf16 string")]
    InvalidUtf16String(#[from] std::string::FromUtf16Error),

    /// Invalid Utf8 OsStr
    #[error("Invalid utf8 os str")]
    InvalidUtf8OsStr,

    /// Invalid Utf8 OsString
    #[error("Invalid utf8 os string")]
    InvalidUtf8OsString { value: OsString },

    /// Failed to parse int
    #[error("Failed to parse int")]
    ParseInt(#[from] std::num::ParseIntError),

    /// The value is missing for some reason
    #[error("Missing value")]
    MissingValue,

    /// This API is unsupported on this platform.
    #[error("Unsupported on this platform")]
    Unsupported,

    /// A generic error occured.
    #[error("{0}")]
    Generic(&'static str),
}

impl Error {
    /// Returns `true` is this is `Error::Unsupported`.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Error::Unsupported)
    }
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

    /// Start measuring.
    ///
    /// Some stats require this to be called along with `end` before they can function.
    /// There must be a delay of at least 1 second between `start` and `end`.
    ///
    /// # Blocking
    /// This is blocking.
    pub fn start(&self) -> Result<(), Error> {
        self.inner.start()
    }

    /// End measuring.
    ///
    /// Some stats require this to be called along with `start` before they can function.
    /// There must be a delay of at least 1 second between `start` and `end`.
    ///
    /// # Blocking
    /// This is blocking.
    pub fn end(&self) -> Result<(), Error> {
        self.inner.end()
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

    /// Get the number of physical cpu cores.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn count_physical_cpus(&self) -> Result<usize, Error> {
        self.inner.count_physical_cpus()
    }

    /// Get the CPU frequency, in hertz.
    ///
    /// This includes CPU TurboBoost.
    ///
    /// # Platform-specific behavior
    /// On Linux, this value may or may not include TurboBoost depending on the driver.
    ///
    /// # Data Collection
    /// This API requires that you call `start` and `end` before calling this.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_cpu_frequency(&self) -> Result<u64, Error> {
        // TODO: Add guard to prevent usage before `start` and `end`?
        self.inner.get_cpu_frequency()
    }

    /// Get the minimum CPU frequency, in hertz.
    ///
    /// This includes CPU TurboBoost.
    ///
    /// # Platform-specific behavior
    /// On Windows, this function is unsupported.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_min_cpu_frequency(&self) -> Result<u64, Error> {
        self.inner.get_min_cpu_frequency()
    }

    /// Get the maximum CPU frequency, in hertz.
    ///
    /// This includes CPU TurboBoost.
    ///
    /// # Platform-specific behavior
    /// On Windows, this value will NOT take into account TurboBoost.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_max_cpu_frequency(&self) -> Result<u64, Error> {
        self.inner.get_max_cpu_frequency()
    }

    /// Get the cpu usage, as a percent.
    ///
    /// # Collection
    /// This API requires that you call `start` and `end` before calling this.
    ///
    /// # Blocking
    /// This is NOT blocking.
    pub fn get_cpu_usage(&self) -> Result<f64, Error> {
        self.inner.get_cpu_usage()
    }

    // TODO: pub fn get_cpu_temperature(&self) -> Result<f64, Error> {
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

    const BLOCKING_DURATION: Duration = Duration::from_millis(1);

    #[test]
    fn boot_time() {
        let cache_context = CacheContext::new();
        let offset = match time::UtcOffset::current_local_offset() {
            Ok(offset) => offset,
            Err(error) => {
                println!("Failed to get local offset ({error}), using UTC...");
                time::UtcOffset::UTC
            }
        };

        let start = Instant::now();
        let boot_time = cache_context
            .get_boot_time()
            .expect("Failed to get boot time");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

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
        let uptime = cache_context.get_uptime().expect("Failed to get uptime");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Uptime: {uptime:?}\nTime: {elapsed:?}");
    }

    #[test]
    fn hostname() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let hostname = cache_context
            .get_hostname()
            .expect("Failed to get hostname");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Hostname: {hostname}\nTime: {elapsed:?}");
    }

    #[test]
    fn architecture() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let arch = cache_context
            .get_architecture()
            .expect("Failed to get arch");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!(
            "Arch: {}\nTime: {:?}",
            arch.map(|arch| arch.as_str()).unwrap_or("Unknown"),
            elapsed
        );
    }

    #[test]
    fn system_name() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let system_name = cache_context
            .get_system_name()
            .expect("Failed to get system name");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("System Name: {system_name:?}\nTime: {elapsed:?}");
    }

    #[test]
    fn system_version() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let system_version = cache_context
            .get_system_version()
            .expect("Failed to get system version");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("System Version: {system_version:?}\nTime: {elapsed:?}");
    }

    #[test]
    fn total_memory() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let total_memory = cache_context
            .get_total_memory()
            .expect("Failed to get total memory");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Total Memory: {total_memory}\nTime: {elapsed:?}");
    }

    #[test]
    fn available_memory() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let available_memory = cache_context
            .get_available_memory()
            .expect("Failed to get available memory");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Available Memory: {available_memory}\nTime: {elapsed:?}");
    }

    #[test]
    fn total_swap() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let total_swap = cache_context
            .get_total_swap()
            .expect("Failed to get total swap");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Total Swap: {total_swap}\nTime: {elapsed:?}");
    }

    #[test]
    fn available_swap() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let available_swap = cache_context
            .get_available_swap()
            .expect("Failed to get available swap memory");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Available Swap: {available_swap}\nTime: {elapsed:?}");
    }

    #[test]
    fn logical_cpus() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let logical_cpus = cache_context
            .count_logical_cpus()
            .expect("Failed to get count logical cpus");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Logical Cpus: {logical_cpus}\nTime: {elapsed:?}");
    }

    #[test]
    fn physical_cpus() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        let physical_cpus = cache_context
            .count_physical_cpus()
            .expect("Failed to get count physical cpus");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Physical Cpus: {physical_cpus}\nTime: {elapsed:?}");
    }

    #[test]
    fn get_cpu_frequency() {
        let cache_context = CacheContext::new();
        cache_context.start().expect("Failed to start");
        std::thread::sleep(Duration::from_secs(1));
        cache_context.end().expect("Failed to end");

        let start = Instant::now();
        let cpu_frequency = cache_context
            .get_cpu_frequency()
            .expect("Failed to get CPU frequency");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("CPU Frequency: {cpu_frequency}\nTime: {elapsed:?}");
    }

    // Unsupported on Windows
    #[test]
    #[cfg(not(windows))]
    fn get_min_cpu_frequency() {
        let cache_context = CacheContext::new();

        let start = Instant::now();
        let min_cpu_frequency = cache_context
            .get_min_cpu_frequency()
            .expect("Failed to get min CPU frequency");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Min CPU Frequency: {min_cpu_frequency}\nTime: {elapsed:?}");
    }

    #[test]
    fn get_max_cpu_frequency() {
        let cache_context = CacheContext::new();

        let start = Instant::now();
        let max_cpu_frequency = cache_context
            .get_max_cpu_frequency()
            .expect("Failed to get max CPU frequency");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("Max CPU Frequency: {max_cpu_frequency}\nTime: {elapsed:?}");
    }

    #[test]
    fn get_cpu_usage() {
        let cache_context = CacheContext::new();
        let start = Instant::now();
        cache_context.start().expect("Failed to start");
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(200),
            "Blocked for {elapsed:?}"
        );

        std::thread::sleep(Duration::from_secs(1));
        let start = Instant::now();
        cache_context.end().expect("Failed to end");
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(200));

        let start = Instant::now();
        let cpu_usage = cache_context
            .get_cpu_usage()
            .expect("Failed to get CPU usage");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        println!("CPU Usage: {cpu_usage}\nTime: {elapsed:?}");
    }
}
