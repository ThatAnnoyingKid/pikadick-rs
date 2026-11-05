use std::{
    ffi::OsString,
    fmt::Write,
    mem::MaybeUninit,
    os::windows::ffi::OsStringExt,
    time::Duration,
};
use windows_sys::Win32::System::{
    SystemInformation::{
        COMPUTER_NAME_FORMAT,
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
    Threading::GetActiveProcessorCount,
    WindowsProgramming::{
        MAX_COMPUTERNAME_LENGTH,
        uaw_wcslen,
    },
};

/// A wide cstr
pub struct WCStr {
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
    pub unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a WCStr {
        unsafe {
            let len = uaw_wcslen(ptr);
            let slice = std::slice::from_raw_parts(ptr, len + 1);
            Self::from_bytes_with_nul_unchecked(slice)
        }
    }

    /// # Safety
    /// * The slice must contain only one nul
    /// * The nul must be the last element in the slice
    unsafe fn from_bytes_with_nul_unchecked(slice: &[u16]) -> &Self {
        unsafe { &*(slice as *const [u16] as *const Self) }
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

/// A wrapper for `GetTickCount64`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64
pub fn get_tick_count_64() -> Duration {
    Duration::from_millis(unsafe { GetTickCount64() })
}

/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format
#[expect(dead_code)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ComputerNameFormat {
    NetBios,
    DnsHostname,
    DnsDomain,
    DnsFullyQualified,
    PhysicalNetBIOS,
    PhysicalDnsHostname,
    PhysicalDnsDomain,
    PhysicalDnsFullyQualified,
}

impl ComputerNameFormat {
    fn as_raw(self) -> COMPUTER_NAME_FORMAT {
        match self {
            Self::NetBios => ComputerNameNetBIOS,
            Self::DnsHostname => ComputerNameDnsHostname,
            Self::DnsDomain => ComputerNameDnsDomain,
            Self::DnsFullyQualified => ComputerNameDnsFullyQualified,
            Self::PhysicalNetBIOS => ComputerNamePhysicalNetBIOS,
            Self::PhysicalDnsHostname => ComputerNamePhysicalDnsHostname,
            Self::PhysicalDnsDomain => ComputerNamePhysicalDnsDomain,
            Self::PhysicalDnsFullyQualified => ComputerNamePhysicalDnsFullyQualified,
        }
    }
}

/// A wrapper for `GetComputerNameExW`.
///
/// See https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw
pub fn get_computer_name(computer_name_format: ComputerNameFormat) -> std::io::Result<OsString> {
    const MAX_COMPUTERNAME_LENGTH_USIZE: usize = MAX_COMPUTERNAME_LENGTH as usize;

    let buffer: &mut [std::mem::MaybeUninit<u16>] =
        &mut [MaybeUninit::uninit(); MAX_COMPUTERNAME_LENGTH_USIZE];
    let mut size = MAX_COMPUTERNAME_LENGTH;

    let code = unsafe {
        GetComputerNameExW(
            computer_name_format.as_raw(),
            buffer.as_mut_ptr().cast::<u16>(),
            &mut size,
        )
    };
    if code == 0 {
        return Err(std::io::Error::last_os_error());
    }

    // The max len is MAX_COMPUTERNAME_LENGTH (15).
    // This is smaller than any possible usize.
    // Therefore, this conversion will never fail.
    let len = usize::try_from(size).unwrap();
    let slice: &[u16] = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast(), len) };

    Ok(OsString::from_wide(slice))
}

/// A wrapper for `GetActiveProcessorCount`.
///
/// Use `u16::MAX` to get all processor groups.
///
/// See `https://docs.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getactiveprocessorcount`.
pub fn get_active_processor_count(group_number: u16) -> std::io::Result<u32> {
    let code = unsafe { GetActiveProcessorCount(group_number) };
    if code == 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(code)
}
