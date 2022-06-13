use super::Error;
use nix::sys::sysinfo::sysinfo;
use std::time::SystemTime;

pub fn get_boot_time() -> Result<SystemTime, Error> {
    let sysinfo = sysinfo().map_err(std::io::Error::from)?;
    Ok(SystemTime::now() - sysinfo.uptime())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::{
        Duration,
        Instant,
    };

    #[test]
    fn sysinfo_does_not_block() {
        let start = Instant::now();
        let _sysinfo = sysinfo();
        assert!(start.elapsed() < Duration::from_millis(1));
    }
}
