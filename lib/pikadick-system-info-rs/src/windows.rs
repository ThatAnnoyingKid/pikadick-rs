use super::Error;
use std::time::{
    Duration,
    SystemTime,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;

/// Get the time the system was booted
pub fn get_boot_time() -> Result<SystemTime, Error> {
    Ok(SystemTime::now() - get_tick_count_64())
}

/// A wrapper for `GetTickCount64`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64
fn get_tick_count_64() -> Duration {
    Duration::from_millis(unsafe { GetTickCount64() })
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    #[test]
    fn get_tick_count_64_does_not_block() {
        let start = Instant::now();
        let _boot_time = get_boot_time();
        assert!(start.elapsed() < Duration::from_millis(4));
    }
}
