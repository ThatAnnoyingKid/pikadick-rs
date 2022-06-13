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

use std::time::SystemTime;

/// The library error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An io error occured
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Get the boot time.
pub fn get_boot_time() -> Result<SystemTime, Error> {
    imp::get_boot_time()
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
                eprintln!("failed to get local offset ({error}), using UTC...");
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
}
