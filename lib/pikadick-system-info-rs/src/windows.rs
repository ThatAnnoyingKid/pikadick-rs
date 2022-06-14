use super::Error;
use std::time::{
    Duration,
    SystemTime,
};
use windows_sys::Win32::System::{
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
        GetTickCount64,
    },
    WindowsProgramming::MAX_COMPUTERNAME_LENGTH,
};

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
}
