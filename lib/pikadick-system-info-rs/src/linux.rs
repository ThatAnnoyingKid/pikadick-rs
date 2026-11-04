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

#[derive(Debug)]
struct CpuFreqInfo {
    freq: u64,
    min_freq: u64,
    max_freq: u64,
}

fn get_cpu_freq_info() -> Result<CpuFreqInfo, Error> {
    // Inspired by: https://unix.stackexchange.com/questions/87522/why-do-cpuinfo-cur-freq-and-proc-cpuinfo-report-different-numbers

    let mut frequencies = Vec::with_capacity(4);
    let mut min_frequencies = Vec::with_capacity(4);
    let mut max_frequencies = Vec::with_capacity(4);
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

        let min_freq_path = entry_path.join("cpufreq/cpuinfo_min_freq");
        let min_freq_string = std::fs::read_to_string(min_freq_path)?;
        let min_cpu_freq: u64 = min_freq_string.trim().parse()?;
        min_frequencies.push(min_cpu_freq);

        let max_freq_path = entry_path.join("cpufreq/cpuinfo_max_freq");
        let max_freq_string = std::fs::read_to_string(max_freq_path)?;
        let max_cpu_freq: u64 = max_freq_string.trim().parse()?;
        max_frequencies.push(max_cpu_freq);
    }
    if frequencies.is_empty() {
        return Err(Error::Generic("Failed to get CPU frequencies"));
    }
    let cpu_count =
        f64::from(u32::try_from(frequencies.len()).expect("CPU count cannot fit in a u32"));

    let curr_sum: f64 = frequencies.iter().map(|value| *value as f64).sum();
    let curr_freq_khz = curr_sum / cpu_count;
    let curr_freq_hz = curr_freq_khz * 1_000.0;

    let min_sum: f64 = min_frequencies.iter().map(|value| *value as f64).sum();
    let min_freq_khz = min_sum / cpu_count;
    let min_freq_hz = min_freq_khz * 1_000.0;

    let max_sum: f64 = max_frequencies.iter().map(|value| *value as f64).sum();
    let max_freq_khz = max_sum / cpu_count;
    let max_freq_hz = max_freq_khz * 1_000.0;

    Ok(CpuFreqInfo {
        freq: curr_freq_hz as u64,
        min_freq: min_freq_hz as u64,
        max_freq: max_freq_hz as u64,
    })
}

#[derive(Debug)]
struct Stat {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
    #[expect(dead_code)]
    guest: u64,
    #[expect(dead_code)]
    guest_nice: u64,
}

impl Stat {
    /// Get the total time.
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    /// Get the idle time.
    fn idle(&self) -> u64 {
        self.idle + self.iowait
    }
}

fn get_stat() -> Result<Stat, Error> {
    const MISSING_PART_ERROR: &str = "Missing stat part";

    let stat_string = std::fs::read_to_string("/proc/stat")?;
    let (line, _) = stat_string
        .split_once('\n')
        .ok_or(Error::Generic("Missing newline"))?;

    let mut iter = line.split(' ').filter(|value| !value.is_empty());
    let cpu = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    if cpu != "cpu" {
        return Err(Error::Generic("Invalid cpu part"));
    }

    let user = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let user: u64 = user.parse()?;

    let nice = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let nice: u64 = nice.parse()?;

    let system = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let system: u64 = system.parse()?;

    let idle = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let idle: u64 = idle.parse()?;

    let iowait = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let iowait: u64 = iowait.parse()?;

    let irq = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let irq: u64 = irq.parse()?;

    let softirq = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let softirq: u64 = softirq.parse()?;

    let steal = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let steal: u64 = steal.parse()?;

    let guest = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let guest: u64 = guest.parse()?;

    let guest_nice = iter.next().ok_or(Error::Generic(MISSING_PART_ERROR))?;
    let guest_nice: u64 = guest_nice.parse()?;

    if iter.next().is_some() {
        return Err(Error::Generic("Unexpected stat part"));
    }

    Ok(Stat {
        user,
        nice,
        system,
        idle,
        iowait,
        irq,
        softirq,
        steal,
        guest,
        guest_nice,
    })
}

fn calc_cpu_usage(start: &Stat, end: &Stat) -> Result<f64, Error> {
    let start_total = start.total();
    let start_idle = start.idle();

    let end_total = end.total();
    let end_idle = end.idle();

    let total = (end_total - start_total) as f64;
    let idle = (end_idle - start_idle) as f64;

    if total == 0.0 {
        return Err(Error::Generic("total was 0"));
    }

    Ok(100.0 * (1.0 - (idle / total)))
}

#[derive(Debug)]
struct ThermalZone {
    name: String,
    value: u64,
}

fn get_thermal_zones() -> Result<Vec<ThermalZone>, Error> {
    let mut zones = Vec::with_capacity(4);
    for entry in std::fs::read_dir("/sys/class/thermal/")? {
        let entry = entry?;
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Generic("File name is not utf8"))?;
        let file_name = match file_name.strip_prefix("thermal_zone") {
            Some(name) => name,
            None => continue,
        };
        let _n: u64 = file_name.parse()?;

        let entry_path = entry.path();
        let temp_path = entry_path.join("temp");
        let temp_string = std::fs::read_to_string(temp_path)?;
        let temp: u64 = temp_string.trim().parse()?;

        let type_path = entry_path.join("type");
        let type_string = std::fs::read_to_string(type_path)?;

        zones.push(ThermalZone {
            name: type_string,
            value: temp,
        });
    }

    Ok(zones)
}

#[derive(Debug)]
struct StatInfo {
    start: Stat,
    end: OnceCell<Stat>,
}

/// A cache for system data
#[derive(Debug)]
pub(crate) struct CacheContext {
    sysinfo: OnceCell<SysInfo>,
    utsname: OnceCell<UtsName>,
    cpu_info: OnceCell<Vec<CpuInfo>>,
    cpu_freq_info: OnceCell<CpuFreqInfo>,
    stat_info: OnceCell<StatInfo>,
    thermal_zones: OnceCell<Vec<ThermalZone>>,
}

impl CacheContext {
    /// Make a new cache context.
    pub fn new() -> Self {
        Self {
            sysinfo: OnceCell::new(),
            utsname: OnceCell::new(),
            cpu_info: OnceCell::new(),
            cpu_freq_info: OnceCell::new(),
            stat_info: OnceCell::new(),
            thermal_zones: OnceCell::new(),
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

    /// Get cpu freq info.
    fn get_cpu_freq_info(&self) -> Result<&CpuFreqInfo, Error> {
        self.cpu_freq_info.get_or_try_init(get_cpu_freq_info)
    }

    /// Get stat info.
    fn get_stat_info(&self) -> Result<&StatInfo, Error> {
        self.stat_info
            .get()
            .ok_or(Error::Generic("Need to call `start`"))
    }

    /// Get thermal zones
    fn get_thermal_zones(&self) -> Result<&[ThermalZone], Error> {
        self.thermal_zones
            .get_or_try_init(get_thermal_zones)
            .map(|value| value.as_slice())
    }

    /// Start measuring.
    pub fn start(&self) -> Result<(), Error> {
        self.stat_info.get_or_try_init(|| {
            let start = get_stat()?;

            Ok::<_, Error>(StatInfo {
                start,
                end: OnceCell::new(),
            })
        })?;
        Ok(())
    }

    /// End measuring.
    pub fn end(&self) -> Result<(), Error> {
        let stat_info = self.get_stat_info()?;
        stat_info.end.get_or_try_init(get_stat)?;
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
                return Err(Error::Generic("Count should be greater than 0"));
            }

            Ok(count)
        }
    }

    /// Get the cpu frequency.
    pub fn get_cpu_frequency(&self) -> Result<u64, Error> {
        let cpu_freq_info = self.get_cpu_freq_info()?;
        Ok(cpu_freq_info.freq)
    }

    /// Get the min cpu frequency.
    pub fn get_min_cpu_frequency(&self) -> Result<u64, Error> {
        let cpu_freq_info = self.get_cpu_freq_info()?;
        Ok(cpu_freq_info.min_freq)
    }

    /// Get the max cpu frequency.
    pub fn get_max_cpu_frequency(&self) -> Result<u64, Error> {
        let cpu_freq_info = self.get_cpu_freq_info()?;
        Ok(cpu_freq_info.max_freq)
    }

    /// Get the cpu usage.
    pub fn get_cpu_usage(&self) -> Result<f64, Error> {
        let stat_info = self.get_stat_info()?;
        let end = stat_info
            .end
            .get()
            .ok_or(Error::Generic("Need to call `start`"))?;

        calc_cpu_usage(&stat_info.start, end)
    }

    /// Get the cpu temperature, in degrees Celsius.
    pub fn get_cpu_temperature(&self) -> Result<f64, Error> {
        let thermal_zones = self.get_thermal_zones()?;

        for zone in thermal_zones {
            if zone.name == "x86_pkg_temp" || zone.name == "cpu-thermal" {
                return Ok((zone.value as f64) / 1000.0);
            }
        }

        Err(Error::MissingValue)
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
        let _cpu_info = get_cpu_info().expect("Failed to get cpu info");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_cpu_freq_info_does_not_block() {
        let start = Instant::now();
        let _cpu_info = get_cpu_info().expect("Failed to get cpu freq info");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_stat_does_not_block() {
        let start = Instant::now();
        let _stat = get_stat().expect("Failed to get stat");
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_thermal_zones_does_not_block() {
        let start = Instant::now();
        let _stat = get_thermal_zones().expect("Failed to get stat");
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
