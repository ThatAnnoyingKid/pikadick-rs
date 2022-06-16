use super::Error;
use platforms::Arch;
use std::time::{
    Duration,
    SystemTime,
};
use windows_sys::Win32::System::{
    Diagnostics::Debug::{
        PROCESSOR_ARCHITECTURE,
        PROCESSOR_ARCHITECTURE_AMD64,
        PROCESSOR_ARCHITECTURE_ARM,
        PROCESSOR_ARCHITECTURE_IA64,
        PROCESSOR_ARCHITECTURE_INTEL,
        PROCESSOR_ARCHITECTURE_UNKNOWN,
    },
    SystemInformation::{
        ComputerNameDnsDomain,
        ComputerNameDnsFullyQualified,
        ComputerNameDnsHostname,
        ComputerNameNetBIOS,
        ComputerNamePhysicalDnsDomain,
        ComputerNamePhysicalDnsFullyQualified,
        ComputerNamePhysicalDnsHostname,
        ComputerNamePhysicalNetBIOS,
        GetComputerNameExW,
        GetNativeSystemInfo,
        GetTickCount64,
        SYSTEM_INFO,
    },
    WindowsProgramming::MAX_COMPUTERNAME_LENGTH,
};

// This is not in `windows_sys`.
const PROCESSOR_ARCHITECTURE_ARM64: PROCESSOR_ARCHITECTURE = 12;

/// Get the time the system was booted
pub fn get_boot_time() -> Result<SystemTime, Error> {
    Ok(SystemTime::now() - get_uptime()?)
}

/// Get the uptime.
pub fn get_uptime() -> Result<Duration, Error> {
    Ok(get_tick_count_64())
}

/// Get the hostname
pub fn get_hostname() -> Result<String, Error> {
    get_computer_name(ComputerNameFormat::PhysicalDnsHostname)
}

/// Get the architecture.
pub fn get_architecture() -> Result<Option<Arch>, Error> {
    let system_info = get_system_info();
    match system_info.processor_architecture() {
        Ok(ProcessorArchitecture::Amd64) => Ok(Some(Arch::X86_64)),
        Ok(ProcessorArchitecture::Arm) => Ok(Some(Arch::Arm)),
        Ok(ProcessorArchitecture::Arm64) => Ok(Some(Arch::AArch64)),
        // Rust doesn't currently support Itanium, so I don't see how we could possibly get this value here?
        Ok(ProcessorArchitecture::Ia64) => Ok(None),
        Ok(ProcessorArchitecture::Intel) => Ok(Some(Arch::X86)),
        Ok(ProcessorArchitecture::Unknown) => Ok(None),
        Err(_e) => Ok(None),
    }
}

/// A wrapper for `GetTickCount64`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64
fn get_tick_count_64() -> Duration {
    Duration::from_millis(unsafe { GetTickCount64() })
}

/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format
#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ComputerNameFormat {
    NetBios = ComputerNameNetBIOS,
    DnsHostname = ComputerNameDnsHostname,
    DnsDomain = ComputerNameDnsDomain,
    DnsFullyQualified = ComputerNameDnsFullyQualified,
    PhysicalNetBIOS = ComputerNamePhysicalNetBIOS,
    PhysicalDnsHostname = ComputerNamePhysicalDnsHostname,
    PhysicalDnsDomain = ComputerNamePhysicalDnsDomain,
    PhysicalDnsFullyQualified = ComputerNamePhysicalDnsFullyQualified,
}

/// A wrapper for `GetComputerNameExA`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexa
fn get_computer_name(computer_name_format: ComputerNameFormat) -> Result<String, Error> {
    const MAX_COMPUTERNAME_LENGTH_USIZE: usize = MAX_COMPUTERNAME_LENGTH as usize;

    let buffer: &mut [std::mem::MaybeUninit<u16>] =
        &mut [std::mem::MaybeUninit::uninit(); MAX_COMPUTERNAME_LENGTH_USIZE];
    let mut size = MAX_COMPUTERNAME_LENGTH;

    let code = unsafe {
        GetComputerNameExW(
            computer_name_format as i32,
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if code != 0 {
        let len = size.try_into().expect("failed to convert len to `usize`");
        let slice: &[u16] = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast(), len) };
        Ok(String::from_utf16(slice)?)
    } else {
        Err(Error::Io(std::io::Error::last_os_error()))
    }
}

/// A wrapper for the wProcessorArchitecture in a `SYSTEM_INFO`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
#[repr(u16)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessorArchitecture {
    Amd64 = PROCESSOR_ARCHITECTURE_AMD64,
    Arm = PROCESSOR_ARCHITECTURE_ARM,
    Arm64 = PROCESSOR_ARCHITECTURE_ARM64,
    Ia64 = PROCESSOR_ARCHITECTURE_IA64,
    Intel = PROCESSOR_ARCHITECTURE_INTEL,
    Unknown = PROCESSOR_ARCHITECTURE_UNKNOWN,
}

/// A wrapper for `SYSTEM_INFO`
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info
#[repr(transparent)]
pub struct SystemInfo(SYSTEM_INFO);

impl SystemInfo {
    /// Get the processor architecture.
    ///
    /// # Returns
    /// If the value is not known to Rust, an Err value is returned.
    pub fn processor_architecture(&self) -> Result<ProcessorArchitecture, u16> {
        let processor_architecture = unsafe { self.0.Anonymous.Anonymous.wProcessorArchitecture };
        match processor_architecture {
            PROCESSOR_ARCHITECTURE_AMD64 => Ok(ProcessorArchitecture::Amd64),
            PROCESSOR_ARCHITECTURE_ARM => Ok(ProcessorArchitecture::Arm),
            PROCESSOR_ARCHITECTURE_ARM64 => Ok(ProcessorArchitecture::Arm64),
            PROCESSOR_ARCHITECTURE_IA64 => Ok(ProcessorArchitecture::Ia64),
            PROCESSOR_ARCHITECTURE_INTEL => Ok(ProcessorArchitecture::Intel),
            PROCESSOR_ARCHITECTURE_UNKNOWN => Ok(ProcessorArchitecture::Unknown),
            _ => Err(processor_architecture),
        }
    }

    /// Get the system page size
    pub fn page_size(&self) -> u32 {
        self.0.dwPageSize
    }
}

impl std::fmt::Debug for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemInfo")
            .field("processor_architecture", &self.processor_architecture())
            .field("page_size", &self.page_size())
            .finish()
    }
}

/// A wrapper for `GetNativeSystemInfo`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo
fn get_system_info() -> SystemInfo {
    let mut raw_system_info = std::mem::MaybeUninit::uninit();
    unsafe {
        GetNativeSystemInfo(raw_system_info.as_mut_ptr());
        SystemInfo(raw_system_info.assume_init())
    }
}

/*
/// A wrapper for `GetLastError`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}
*/

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    #[test]
    fn get_tick_count_64_does_not_block() {
        let start = Instant::now();
        let _boot_time = get_boot_time();
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn get_computer_name_does_not_block() {
        let start = Instant::now();
        let _hostname = get_computer_name(ComputerNameFormat::PhysicalDnsHostname);
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    #[test]
    fn get_system_info_works() {
        let start = Instant::now();
        let _system_info = get_system_info();
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(1));
    }
}
