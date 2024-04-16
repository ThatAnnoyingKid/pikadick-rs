use super::Error;
use nix::{
    sys::{
        sysinfo::{
            sysinfo,
            SysInfo,
        },
        utsname::{
            uname,
            UtsName,
        },
    },
    unistd::{
        gethostname,
        sysconf,
        SysconfVar,
    },
};
use once_cell::sync::OnceCell;
use platforms::Arch;
use std::{
    os::unix::ffi::OsStringExt,
    time::{
        Duration,
        SystemTime,
    },
};

/// A cache for system data
#[derive(Debug)]
pub(crate) struct CacheContext {
    sysinfo: OnceCell<SysInfo>,
    utsname: OnceCell<UtsName>,
}

impl CacheContext {
    /// Make a new cache context
    pub fn new() -> Self {
        Self {
            sysinfo: OnceCell::new(),
            utsname: OnceCell::new(),
        }
    }

    /// Get the system info
    fn get_sysinfo(&self) -> Result<&SysInfo, Error> {
        self.sysinfo
            .get_or_try_init(|| sysinfo().map_err(|error| Error::Io(std::io::Error::from(error))))
    }

    /// Get the utsname
    fn get_utsname(&self) -> Result<&UtsName, Error> {
        self.utsname
            .get_or_try_init(|| uname().map_err(|error| Error::Io(std::io::Error::from(error))))
    }

    /// Get the time the system was booted
    pub fn get_boot_time(&self) -> Result<SystemTime, Error> {
        Ok(SystemTime::now() - self.get_uptime()?)
    }

    /// Get the uptime
    pub fn get_uptime(&self) -> Result<Duration, Error> {
        Ok(self.get_sysinfo()?.uptime())
    }

    /// Get the hostname
    pub fn get_hostname(&self) -> Result<String, Error> {
        let buffer = gethostname().map_err(std::io::Error::from)?.into_vec();
        String::from_utf8(buffer).map_err(Error::InvalidUtf8String)
    }

    /// Get the architecture.
    pub fn get_architecture(&self) -> Result<Option<Arch>, Error> {
        let utsname = self.get_utsname()?;

        // See:
        // * https://en.wikipedia.org/wiki/Uname
        // * https://stackoverflow.com/questions/45125516/possible-values-for-uname-m
        match utsname.machine().to_str().ok_or(Error::InvalidUtf8OsStr)? {
            "x86" | "i386" | "i586" | "i686" => Ok(Some(Arch::X86)),
            "x86_64" | "amd64" => Ok(Some(Arch::X86_64)),
            "arm" | "armv6l" | "armv7" | "armv7l" => Ok(Some(Arch::Arm)),
            "aarch64_be" | "aarch64" | "armv8b" | "armv8l" => Ok(Some(Arch::AArch64)),
            "ppc" => Ok(Some(Arch::PowerPc)),
            "ppc64" | "ppc64le" => Ok(Some(Arch::PowerPc64)),
            "sparc64" => Ok(Some(Arch::Sparc64)),
            _ => Ok(None),
        }
    }

    /// Get the system name.
    pub fn get_system_name(&self) -> Result<Option<String>, Error> {
        let utsname = self.get_utsname()?;
        let name = utsname.sysname().to_str().ok_or(Error::InvalidUtf8OsStr)?;
        let release = utsname.release().to_str().ok_or(Error::InvalidUtf8OsStr)?;

        Ok(Some(format!("{name} {release}")))
    }

    /// Get the system version.
    pub fn get_system_version(&self) -> Result<String, Error> {
        let utsname = self.get_utsname()?;
        let version = utsname.version().to_str().ok_or(Error::InvalidUtf8OsStr)?;

        Ok(version.to_string())
    }

    /// Get the total amount of memory in the computer, in bytes
    pub fn get_total_memory(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.ram_total())
    }

    /// Get the available amount of memory in the computer, in bytes
    pub fn get_available_memory(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.ram_unused())
    }

    /// Get the total amount of swap in the computer, in bytes
    pub fn get_total_swap(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.swap_total())
    }

    /// Get the available amount of swap in the computer, in bytes
    pub fn get_available_swap(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.swap_free())
    }

    /// Get the number of logical cpu cores.
    pub fn count_logical_cpus(&self) -> Result<usize, Error> {
        Ok(sysconf(SysconfVar::_NPROCESSORS_ONLN)
            .map_err(|error| Error::Io(std::io::Error::from(error)))?
            .ok_or(Error::MissingValue)?
            .try_into()
            .expect("the number of cores cannot fit in a `usize`"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    fn assert_impl_send<T>()
    where
        T: Send,
    {
    }

    #[test]
    fn sysinfo_does_not_block() {
        let start = Instant::now();
        let _sysinfo = sysinfo();
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn gethostname_does_not_block() {
        let start = Instant::now();
        let _hostname = gethostname().expect("failed to get hostname");
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn uname_does_not_block() {
        let start = Instant::now();
        let _utsname = uname().expect("failed to get utsname");
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn sysconf_sc_n_processors_onln_does_not_block() {
        let start = Instant::now();
        let _logical_cpus = sysconf_n_processors_onln().expect("failed to get logical cpus");
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn cache_context_is_send() {
        assert_impl_send::<CacheContext>();
    }
}
