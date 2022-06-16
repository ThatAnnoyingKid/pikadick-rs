use super::Error;
use bitflags::bitflags;
use platforms::Arch;
use std::{
    fmt::Write,
    time::{
        Duration,
        SystemTime,
    },
};
use windows_sys::Win32::{
    Foundation::{
        NTSTATUS,
        STATUS_SUCCESS,
    },
    System::{
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
            GetVersionExW,
            OSVERSIONINFOEXW,
            OSVERSIONINFOW,
            SYSTEM_INFO,
        },
        SystemServices::{
            VER_NT_DOMAIN_CONTROLLER,
            VER_NT_SERVER,
            VER_NT_WORKSTATION,
            VER_SUITE_WH_SERVER,
        },
        WindowsProgramming::{
            uaw_wcslen,
            MAX_COMPUTERNAME_LENGTH,
        },
    },
};

// This is not in `windows_sys`.
const PROCESSOR_ARCHITECTURE_ARM64: PROCESSOR_ARCHITECTURE = 12;
#[allow(non_camel_case_types)]
type PRTL_OSVERSIONINFOW = *mut OSVERSIONINFOW;

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(_lpVersionInformation: PRTL_OSVERSIONINFOW) -> NTSTATUS;
}

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

/// Get the system name
pub fn get_system_name() -> Result<Option<String>, Error> {
    let os_version_info = rtl_get_version();

    // https://www.lifewire.com/windows-version-numbers-2625171
    // https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ns-wdm-_osversioninfoexw
    let system_name = match (
        os_version_info.major_version(),
        os_version_info.minor_version(),
        os_version_info.build_number(),
        os_version_info.product_type(),
        os_version_info.suite_mask(),
    ) {
        (10, 0, build, Ok(ProductType::Workstation), _) if build >= 22000 => "Windows 11",
        (10, 0, build, _, _) if build >= 22000 => "Windows Server 2022",
        (10, 0, _, Ok(ProductType::Workstation), _) => "Windows 10",
        (10, 0, _, _, _) => "Windows Server 2016",
        (6, 3, _, Ok(ProductType::Workstation), _) => "Windows 8.1",
        (6, 3, _, _, _) => "Windows Server 2012 R2",
        (6, 2, _, Ok(ProductType::Workstation), _) => "Windows 8",
        (6, 2, _, _, _) => "Windows Server 2012",
        (6, 1, _, Ok(ProductType::Workstation), _) => "Windows 7",
        (6, 1, _, _, _) => "Windows Server 2008 R2",
        (6, 0, _, Ok(ProductType::Workstation), _) => "Windows Vista",
        (6, 0, _, _, _) => "Windows Server 2008",
        (5, 2, _, _, ProductSuite::WH_SERVER) => "Windows Home Server",
        (5, 2, _, Ok(ProductType::Workstation), _) => "Windows XP Professional x64 Edition",
        (5, 2, _, _, _) => "Windows Server 2003",
        (5, 1, _, _, _) => "Windows XP",
        (5, 0, _, _, _) => "Windows 2000",
        _ => return Ok(None),
    };

    Ok(Some(system_name.to_string()))
}

/// Get the os version
pub fn get_system_version() -> Result<String, Error> {
    let os_version_info = rtl_get_version();
    let major_version = os_version_info.major_version();
    let minor_version = os_version_info.minor_version();
    let build_number = os_version_info.build_number();

    Ok(format!("{major_version}.{minor_version}.{build_number}"))
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

/// A wrapper for `OSVERSIONINFOEXW`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexa.
#[repr(transparent)]
struct OsVersionInfoEx(OSVERSIONINFOEXW);

impl OsVersionInfoEx {
    /// The major version of the os
    pub fn major_version(&self) -> u32 {
        self.0.dwMajorVersion
    }

    /// The minor version of the os
    pub fn minor_version(&self) -> u32 {
        self.0.dwMinorVersion
    }

    /// The build number of the os
    pub fn build_number(&self) -> u32 {
        self.0.dwBuildNumber
    }

    /// The latest service pack installed
    pub fn csd_version(&self) -> Option<&WCStr> {
        let w_str = unsafe { WCStr::from_ptr(self.0.szCSDVersion.as_ptr()) };
        if w_str.to_slice().is_empty() {
            None
        } else {
            Some(w_str)
        }
    }

    /// The major version of the latest service pack
    pub fn service_pack_major(&self) -> Option<u16> {
        let service_pack_major = self.0.wServicePackMajor;
        if service_pack_major == 0 {
            None
        } else {
            Some(service_pack_major)
        }
    }

    /// The minor version of the latest service pack
    pub fn service_pack_minor(&self) -> Option<u16> {
        let service_pack_minor = self.0.wServicePackMinor;
        if service_pack_minor == 0 {
            None
        } else {
            Some(service_pack_minor)
        }
    }

    /// Get product suites on the system
    pub fn suite_mask(&self) -> ProductSuite {
        ProductSuite {
            bits: self.0.wSuiteMask,
        }
    }

    /// Get additional info about they system.
    pub fn product_type(&self) -> Result<ProductType, u8> {
        let product_type = self.0.wProductType;
        match u32::from(product_type) {
            VER_NT_DOMAIN_CONTROLLER => Ok(ProductType::DomainController),
            VER_NT_SERVER => Ok(ProductType::Server),
            VER_NT_WORKSTATION => Ok(ProductType::Workstation),
            _ => Err(product_type),
        }
    }
}

impl std::fmt::Debug for OsVersionInfoEx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsVersionInfoEx")
            .field("major_version", &self.major_version())
            .field("minor_version", &self.minor_version())
            .field("build_number", &self.build_number())
            .field("csd_version", &self.csd_version())
            .field("service_pack_major", &self.service_pack_major())
            .field("service_pack_minor", &self.service_pack_minor())
            .field("product_type", &self.product_type())
            .finish()
    }
}

/// A wide cstr
struct WCStr {
    // TODO: use wchar def?
    inner: [u16],
}

impl WCStr {
    /// Make a [`WCStr`] from a raw ptr.
    ///
    /// # Safety
    /// * The memeory behind the pointer must be valid for the lifetime of the returned reference.
    /// * The ptr must be terminated by a nul byte.
    /// * The memory behing the ptr must not be changed while the reference is alive.
    unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a WCStr {
        let len = uaw_wcslen(ptr);
        let slice = std::slice::from_raw_parts(ptr, len + 1);
        Self::from_bytes_with_nul_unchecked(slice)
    }

    /// # Safety
    /// * The slice must contain only one nul
    /// * The nul must be the last element in the slice
    unsafe fn from_bytes_with_nul_unchecked(slice: &[u16]) -> &Self {
        &*(slice as *const [u16] as *const Self)
    }

    /// Get this as a slice, excluding the NUL.
    pub fn to_slice(&self) -> &[u16] {
        let slice = &self.inner;
        &slice[..slice.len() - 1]
    }
}

impl std::fmt::Debug for WCStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for ch in std::char::decode_utf16(self.to_slice().iter().copied()) {
            match ch {
                Ok(ch) => {
                    for ch in ch.escape_default() {
                        f.write_char(ch)?;
                    }
                }
                Err(e) => {
                    write!(f, "\\x{:X}", e.unpaired_surrogate())?;
                }
            }
        }
        write!(f, "\"")
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ProductType {
    DomainController = VER_NT_DOMAIN_CONTROLLER as u8,
    Server = VER_NT_SERVER as u8,
    Workstation = VER_NT_WORKSTATION as u8,
}

bitflags! {
    struct ProductSuite: u16 {
        const WH_SERVER = VER_SUITE_WH_SERVER as u16;
    }
}

/// A wrapper for `GetVersionEx`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversionexa
#[allow(dead_code)]
fn get_version_ex() -> Result<OsVersionInfoEx, Error> {
    const OS_VERSION_INFO_SIZE: u32 = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

    let mut os_version_info: std::mem::MaybeUninit<OSVERSIONINFOEXW> = unsafe {
        let mut os_version_info: std::mem::MaybeUninit<OSVERSIONINFOEXW> =
            std::mem::MaybeUninit::zeroed();
        std::ptr::addr_of_mut!((*os_version_info.as_mut_ptr()).dwOSVersionInfoSize)
            .write(OS_VERSION_INFO_SIZE);
        os_version_info
    };
    let code = unsafe { GetVersionExW(os_version_info.as_mut_ptr().cast()) };

    if code == 0 {
        return Err(Error::Io(std::io::Error::last_os_error()));
    }

    let os_version_info = unsafe { OsVersionInfoEx(os_version_info.assume_init()) };

    Ok(os_version_info)
}

/// A wrapper for `RtlGetVersion`.
///
/// See https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlgetversion
fn rtl_get_version() -> OsVersionInfoEx {
    const OS_VERSION_INFO_SIZE: u32 = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

    let mut os_version_info: std::mem::MaybeUninit<OSVERSIONINFOEXW> = unsafe {
        let mut os_version_info: std::mem::MaybeUninit<OSVERSIONINFOEXW> =
            std::mem::MaybeUninit::zeroed();
        std::ptr::addr_of_mut!((*os_version_info.as_mut_ptr()).dwOSVersionInfoSize)
            .write(OS_VERSION_INFO_SIZE);
        os_version_info
    };
    let code = unsafe { RtlGetVersion(os_version_info.as_mut_ptr().cast()) };

    assert!(code == STATUS_SUCCESS);

    let os_version_info = unsafe { OsVersionInfoEx(os_version_info.assume_init()) };
    os_version_info
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
    fn get_system_info_does_not_block() {
        let start = Instant::now();
        let _system_info = get_system_info();
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(1));
    }

    #[test]
    fn rtl_get_version_ex_works() {
        let start = Instant::now();
        let version = rtl_get_version();
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(1));

        dbg!(version);
    }
}
