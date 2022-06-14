use super::Error;
use nix::{
    sys::sysinfo::sysinfo,
    unistd::{
        gethostname,
        sysconf,
        SysconfVar,
    },
    NixPath,
};
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
    Ok(sysinfo.uptime())
}

/// Get the hostname
pub fn get_hostname() -> Result<String, Error> {
    let hostname_len: usize = sysconf(SysconfVar::HOST_NAME_MAX)
        .map(|len| usize::try_from(len?).ok())
        .map_err(std::io::Error::from)?
        .unwrap_or(255usize)
        + 1usize;

    let mut buffer = vec![0; hostname_len];
    let hostname_c_str = gethostname(&mut buffer).map_err(std::io::Error::from)?;
    let len = hostname_c_str.len();
    buffer.truncate(len);

    String::from_utf8(buffer).map_err(Error::InvalidUtf8String)
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
