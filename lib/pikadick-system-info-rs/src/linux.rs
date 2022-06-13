use super::Error;
use nix::sys::sysinfo::sysinfo;
use std::time::{
    Duration,
    SystemTime,
};

/// Get the time the system was booted
pub fn get_boot_time() -> Result<SystemTime, Error> {
    Ok(SystemTime::now() - get_uptime()?)
}

/// Get the uptime.
pub fn get_uptime() -> Result<Duration, Error> {
    let sysinfo = sysinfo().map_err(std::io::Error::from)?;
    sysinfo.uptime()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    #[test]
    fn sysinfo_does_not_block() {
        let start = Instant::now();
        let _sysinfo = sysinfo();
        assert!(start.elapsed() < Duration::from_millis(1));
    }
}
