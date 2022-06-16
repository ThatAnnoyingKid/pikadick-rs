use super::Error;
use nix::{
    sys::{
        sysinfo::sysinfo,
        utsname::uname,
    },
    unistd::{
        gethostname,
        sysconf,
        SysconfVar,
    },
    NixPath,
};
use platforms::Arch;
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

    let mut buffer = vec![std::mem::MaybeUninit::uninit(); hostname_len];
    let hostname_c_str = gethostname(&mut buffer).map_err(std::io::Error::from)?;
    let len = hostname_c_str.len();
    buffer.truncate(len);

    let initialized_buffer = unsafe {
        let mut buffer = std::mem::ManuallyDrop::new(buffer);
        Vec::from_raw_parts(buffer.as_mut_ptr().cast(), buffer.len(), buffer.capacity())
    };

    String::from_utf8(initialized_buffer).map_err(Error::InvalidUtf8String)
}
/// Get the architecture.
pub fn get_architecture() -> Result<Option<Arch>, Error> {
    let utsname = uname().map_err(std::io::Error::from)?;

    // See:
    // * https://en.wikipedia.org/wiki/Uname
    // * https://stackoverflow.com/questions/45125516/possible-values-for-uname-m
    match utsname.machine().to_str().ok_or(Error::InvalidUtf8OsStr)? {
        "i386" | "i586" | "i686" => Ok(Some(Arch::X86)),
        "x86_64" | "amd64" | "x86" => Ok(Some(Arch::X86_64)),
        "arm" | "armv6l" | "armv7" | "armv7l" => Ok(Some(Arch::Arm)),
        "aarch64_be" | "aarch64" | "armv8b" | "armv8l" => Ok(Some(Arch::Arm)),
        "ppc" => Ok(Some(Arch::PowerPc)),
        "ppc64" | "ppc64le" => Ok(Some(Arch::PowerPc64)),
        "sparc64" => Ok(Some(Arch::Sparc64)),
        _ => Ok(None),
    }
}

/// Get the system name.
pub fn get_system_name() -> Result<Option<String>, Error> {
    let utsname = uname().map_err(std::io::Error::from)?;
    Ok(Some(
        utsname
            .sysname()
            .to_str()
            .ok_or(Error::InvalidUtf8OsStr)?
            .to_string(),
    ))
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

    #[test]
    fn gethostname_does_not_block() {
        let start = Instant::now();
        let hostname_len: usize = sysconf(SysconfVar::HOST_NAME_MAX)
            .expect("failed to get hostname len")
            .unwrap_or(255)
            .try_into()
            .expect("failed to convert to usize");
        let mut buffer = vec![std::mem::MaybeUninit::uninit(); hostname_len];
        let _hostname = gethostname(&mut buffer).expect("failed to get hostname");
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn uname_does_not_block() {
        let start = Instant::now();
        let _utsname = uname().expect("failed to get utsname");
        assert!(start.elapsed() < Duration::from_millis(1));
    }
}
