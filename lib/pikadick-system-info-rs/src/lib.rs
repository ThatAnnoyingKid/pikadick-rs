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
}

/// Get the boot time.
pub fn get_boot_time() -> Result<SystemTime, Error> {
    imp::get_boot_time()
}

/// Get the uptime.
pub fn get_uptime() -> Result<Duration, Error> {
    imp::get_uptime()
}

/// Get the hostname
pub fn get_hostname() -> Result<String, Error> {
    imp::get_hostname()
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
        let boot_time = get_boot_time().expect("failed to get boot time");
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
        let uptime = get_uptime().expect("failed to get uptime");
        let elapsed = start.elapsed();
        
        println!(
            "Uptime: {:?}\nTime: {:?}",
            uptime,
            elapsed
        );
    }

    #[test]
    fn hostname() {
        let start = Instant::now();
        let hostname = get_hostname().expect("failed to get hostname");
        let elapsed = start.elapsed();

        println!("Hostname: {}\nTime: {:?}", hostname, elapsed);
    }
}
