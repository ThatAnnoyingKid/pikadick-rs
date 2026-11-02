use super::Error;
use nix::{
    sys::{
        sysinfo::{
            SysInfo,
            sysinfo,
        },
        utsname::{
            UtsName,
            uname,
        },
    },
    unistd::{
        SysconfVar,
        gethostname,
        sysconf,
    },
};
use once_cell::sync::OnceCell;
use platforms::Arch;
use std::{
    collections::HashSet,
    os::unix::ffi::OsStringExt,
    time::{
        Duration,
        SystemTime,
    },
};

const DUPLICATE_CPU_INFO_KEY_ERROR: &str = "Duplicate key";

#[derive(Debug)]
struct CpuInfo {
    processor: Option<u32>,
    physical_id: Option<u32>,
}

impl CpuInfo {
    fn new() -> Self {
        Self {
            processor: None,
            physical_id: None,
        }
    }
}

macro_rules! read_cpu_info_keys {
    ($key:ident, $value:ident, $cpu_info:ident {
        $(
            $key_name:literal => $field:ident : $type_:ty
        ),* $(,)?
    }) => {
        match $key {
            $(
                $key_name => {
                    let parsed: $type_ = $value.parse()?;

                    if $cpu_info.$field.is_some() {
                        return Err(Error::Generic(DUPLICATE_CPU_INFO_KEY_ERROR));
                    }

                    $cpu_info.$field = Some(parsed);
                }
            )*
            _ => {}
        }
    };
}

fn parse_cpu_info(input: &str) -> Result<Vec<CpuInfo>, Error> {
    let mut cpu_infos = Vec::new();
    let mut current = Some(CpuInfo::new());
    for line in input.split('\n') {
        if line.is_empty() {
            if let Some(info) = current.take() {
                cpu_infos.push(info);
            }
            continue;
        }

        let (key, value) = line
            .split_once(":")
            .ok_or(Error::Generic("Line missing \":\""))?;
        let key = key.trim();
        let value = value.trim();
        let current = current.get_or_insert(CpuInfo::new());

        read_cpu_info_keys!(key, value, current {
            "processor" => processor: u32,
            "physical id" => physical_id: u32,
        });
    }

    Ok(cpu_infos)
}

fn get_cpu_info() -> Result<Vec<CpuInfo>, Error> {
    let file = std::fs::read_to_string("/proc/cpuinfo")?;
    parse_cpu_info(&file)
}

/// A cache for system data
#[derive(Debug)]
pub(crate) struct CacheContext {
    sysinfo: OnceCell<SysInfo>,
    utsname: OnceCell<UtsName>,
    cpu_info: OnceCell<Vec<CpuInfo>>,
}

impl CacheContext {
    /// Make a new cache context.
    pub fn new() -> Self {
        Self {
            sysinfo: OnceCell::new(),
            utsname: OnceCell::new(),
            cpu_info: OnceCell::new(),
        }
    }

    /// Get the system info.
    fn get_sysinfo(&self) -> Result<&SysInfo, Error> {
        self.sysinfo
            .get_or_try_init(|| sysinfo().map_err(|error| Error::Io(std::io::Error::from(error))))
    }

    /// Get the utsname.
    fn get_utsname(&self) -> Result<&UtsName, Error> {
        self.utsname
            .get_or_try_init(|| uname().map_err(|error| Error::Io(std::io::Error::from(error))))
    }

    /// Get cpu info.
    fn get_cpu_info(&self) -> Result<&[CpuInfo], Error> {
        self.cpu_info
            .get_or_try_init(get_cpu_info)
            .map(|value| value.as_slice())
    }

    /// Start measuring.
    pub fn start(&self) -> Result<(), Error> {
        Ok(())
    }

    /// End measuring.
    pub fn end(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Get the time the system was booted.
    pub fn get_boot_time(&self) -> Result<SystemTime, Error> {
        Ok(SystemTime::now() - self.get_uptime()?)
    }

    /// Get the uptime.
    pub fn get_uptime(&self) -> Result<Duration, Error> {
        Ok(self.get_sysinfo()?.uptime())
    }

    /// Get the hostname.
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

    /// Get the total amount of memory in the computer, in bytes.
    pub fn get_total_memory(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.ram_total())
    }

    /// Get the available amount of memory in the computer, in bytes.
    pub fn get_available_memory(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.ram_unused())
    }

    /// Get the total amount of swap in the computer, in bytes.
    pub fn get_total_swap(&self) -> Result<u64, Error> {
        let sysinfo = self.get_sysinfo()?;
        Ok(sysinfo.swap_total())
    }

    /// Get the available amount of swap in the computer, in bytes.
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
            .expect("The number of cores cannot fit in a `usize`"))
    }

    /// Get the number of physical cpu cores.
    pub fn count_physical_cpus(&self) -> Result<usize, Error> {
        let cpu_info = self.get_cpu_info()?;

        // Some computers, like the Raspberry Pi 4, will not report the physical_id field.
        // In these cases, fallback to counting based on the processor field.
        let has_physical_ids = cpu_info.iter().any(|info| info.physical_id.is_some());
        if has_physical_ids {
            let physical_ids: HashSet<u32> = cpu_info
                .iter()
                .map(|info| {
                    info.physical_id
                        .ok_or(Error::Generic("Missing physical id"))
                })
                .collect::<Result<_, _>>()?;
            Ok(physical_ids.len())
        } else {
            let processors: HashSet<u32> = cpu_info
                .iter()
                // The Raspberry Pi 4 will report the BCM2835 here (why?),
                // which does not have a processor field.
                .filter_map(|info| info.processor)
                .collect();
            let count = processors.len();
            if count == 0 {
                return Err(Error::Generic("count should be greater than 0"));
            }

            Ok(count)
        }
    }

    /// Get the cpu frequency.
    pub fn get_cpu_frequency(&self) -> Result<u64, Error> {
        // Inspired by: https://unix.stackexchange.com/questions/87522/why-do-cpuinfo-cur-freq-and-proc-cpuinfo-report-different-numbers

        let mut frequencies = Vec::with_capacity(4);
        for entry in std::fs::read_dir("/sys/devices/system/cpu/")? {
            let entry = entry?;
            let file_name = entry
                .file_name()
                .to_os_string()
                .into_string()
                .map_err(|_| Error::Generic("File name is not utf8"))?;
            let trimmed_file_name = match file_name.strip_prefix("cpu") {
                Some(value) => value,
                None => continue,
            };
            let _cpu_n = match trimmed_file_name.parse::<u32>() {
                Ok(value) => value,
                Err(_error) => continue,
            };

            let entry_path = entry.path();
            let cpuinfo_cur_freq_path = entry_path.join("cpufreq/cpuinfo_cur_freq");
            let scaling_cur_freq_path = entry_path.join("cpufreq/scaling_cur_freq");
            let cpu_freq_string = std::fs::read_to_string(cpuinfo_cur_freq_path)
                .or_else(|_| std::fs::read_to_string(scaling_cur_freq_path))?;
            let cpu_freq: u64 = cpu_freq_string.trim().parse()?;

            frequencies.push(cpu_freq);
        }
        if frequencies.is_empty() {
            return Err(Error::Generic("Failed to get cpu frequencies"));
        }
        let sum: f64 = frequencies.iter().map(|value| *value as f64).sum();
        let count =
            f64::from(u32::try_from(frequencies.len()).expect("Cpu count cannot fit in a u32"));
        let freq_khz = sum / count;
        let freq_hz = freq_khz * 1_000.0;

        Ok(freq_hz as u64)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    const BLOCKING_DURATION: Duration = Duration::from_millis(1);

    fn assert_impl_send<T>()
    where
        T: Send,
    {
    }

    fn assert_impl_sync<T>()
    where
        T: Sync,
    {
    }

    #[test]
    fn sysinfo_does_not_block() {
        let start = Instant::now();
        let _sysinfo = sysinfo();
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn gethostname_does_not_block() {
        let start = Instant::now();
        let _hostname = gethostname().expect("Failed to get hostname");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn uname_does_not_block() {
        let start = Instant::now();
        let _utsname = uname().expect("Failed to get utsname");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn sysconf_sc_n_processors_onln_does_not_block() {
        let start = Instant::now();
        let _logical_cpus = sysconf(SysconfVar::_NPROCESSORS_ONLN)
            .expect("Failed to get logical cpus")
            .expect("Missing logical cpus");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_cpu_info_does_not_block() {
        let start = Instant::now();
        let _cpu_info = get_cpu_info().expect("Failed to parse cpu info");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn cache_context_is_send() {
        assert_impl_send::<CacheContext>();
    }

    #[test]
    fn cache_context_is_sync() {
        assert_impl_sync::<CacheContext>();
    }
}
