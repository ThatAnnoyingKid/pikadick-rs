mod util;

use self::util::{
    WCStr,
    get_tick_count_64,
};
use super::Error;
use crate::windows::util::{
    ComputerNameFormat,
    get_active_processor_count,
    get_computer_name,
};
use bitflags::bitflags;
use core::{
    any::Any,
    ffi::c_void,
    mem::MaybeUninit,
    time::Duration,
};
use once_cell::sync::OnceCell;
use platforms::Arch;
use std::{
    marker::PhantomData,
    sync::Mutex,
    time::SystemTime,
};
use windows_sys::{
    Wdk::System::SystemServices::RtlGetVersion,
    Win32::{
        Foundation::{
            ERROR_INSUFFICIENT_BUFFER,
            ERROR_SUCCESS,
            FALSE,
            RtlNtStatusToDosError,
            STATUS_SUCCESS,
            TRUE,
        },
        System::{
            Performance::{
                PDH_CSTATUS_NEW_DATA,
                PDH_CSTATUS_VALID_DATA,
                PDH_FMT_COUNTERVALUE,
                PDH_FMT_COUNTERVALUE_ITEM_W,
                PDH_FMT_DOUBLE,
                PDH_HCOUNTER,
                PDH_HQUERY,
                PDH_MORE_DATA,
                PDH_RAW_COUNTER_ITEM_W,
                PdhAddEnglishCounterW,
                PdhCloseQuery,
                PdhCollectQueryData,
                PdhGetFormattedCounterArrayW,
                PdhGetFormattedCounterValue,
                PdhGetRawCounterArrayW,
                PdhOpenQueryW,
            },
            Power::{
                CallNtPowerInformation,
                PROCESSOR_POWER_INFORMATION,
                ProcessorInformation,
            },
            ProcessStatus::{
                ENUM_PAGE_FILE_INFORMATION,
                EnumPageFilesW,
                GetPerformanceInfo,
                PERFORMANCE_INFORMATION,
            },
            SystemInformation::{
                GetLogicalProcessorInformationEx,
                GetNativeSystemInfo,
                GetVersionExW,
                GlobalMemoryStatusEx,
                LOGICAL_PROCESSOR_RELATIONSHIP,
                MEMORYSTATUSEX,
                OSVERSIONINFOEXW,
                PROCESSOR_ARCHITECTURE_AMD64,
                PROCESSOR_ARCHITECTURE_ARM,
                PROCESSOR_ARCHITECTURE_ARM64,
                PROCESSOR_ARCHITECTURE_IA64,
                PROCESSOR_ARCHITECTURE_INTEL,
                PROCESSOR_ARCHITECTURE_UNKNOWN,
                RelationProcessorCore,
                SYSTEM_INFO,
                SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
            },
            SystemServices::{
                VER_NT_DOMAIN_CONTROLLER,
                VER_NT_SERVER,
                VER_NT_WORKSTATION,
                VER_SUITE_WH_SERVER,
            },
        },
    },
    core::{
        BOOL,
        PCWSTR,
    },
};

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

    /// Get the number of processors.
    pub fn number_of_processors(&self) -> u32 {
        self.0.dwNumberOfProcessors
    }
}

impl std::fmt::Debug for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemInfo")
            .field("processor_architecture", &self.processor_architecture())
            .field("page_size", &self.page_size())
            .field("number_of_processors", &self.number_of_processors())
            .finish()
    }
}

// This is essentially just bytes.
// Even though it has a pointer in it, it doesn't point to valid memory and simply marks the start and end of the address space.
unsafe impl Send for SystemInfo {}

// This is essentially just bytes.
// Even though it has a pointer in it, it doesn't point to valid memory and simply marks the start and end of the address space.
unsafe impl Sync for SystemInfo {}

/// A wrapper for `GetNativeSystemInfo`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo
fn get_system_info() -> SystemInfo {
    let mut raw_system_info = MaybeUninit::uninit();
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
        ProductSuite::from_bits_retain(self.0.wSuiteMask)
    }

    /// Get additional info about the system.
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

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ProductType {
    DomainController = VER_NT_DOMAIN_CONTROLLER as u8,
    Server = VER_NT_SERVER as u8,
    Workstation = VER_NT_WORKSTATION as u8,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
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

    let mut os_version_info: MaybeUninit<OSVERSIONINFOEXW> = unsafe {
        let mut os_version_info: MaybeUninit<OSVERSIONINFOEXW> = MaybeUninit::zeroed();
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

    let mut os_version_info: MaybeUninit<OSVERSIONINFOEXW> = unsafe {
        let mut os_version_info: MaybeUninit<OSVERSIONINFOEXW> = MaybeUninit::zeroed();
        std::ptr::addr_of_mut!((*os_version_info.as_mut_ptr()).dwOSVersionInfoSize)
            .write(OS_VERSION_INFO_SIZE);
        os_version_info
    };
    let code = unsafe { RtlGetVersion(os_version_info.as_mut_ptr().cast()) };

    assert!(code == STATUS_SUCCESS);

    unsafe { OsVersionInfoEx(os_version_info.assume_init()) }
}

/// A wrapper for `MEMORYSTATUSEX`.
///
/// See `https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex`
struct MemoryStatusEx(MEMORYSTATUSEX);

impl MemoryStatusEx {
    /// Get the percentage of memory in use, on a scale from 0 to 100
    pub fn memory_load(&self) -> u8 {
        u8::try_from(self.0.dwMemoryLoad).expect("`dwMemoryLoad` cannot fit in a `u8`")
    }

    /// The total amount of physical memory in bytes
    pub fn total_physical(&self) -> u64 {
        self.0.ullTotalPhys
    }

    /// The total amount of available physical memory in bytes
    pub fn available_physical(&self) -> u64 {
        self.0.ullAvailPhys
    }

    /// The max available memory (with page file) size for the process or computer, whichever is smaller.
    pub fn total_page_file(&self) -> u64 {
        self.0.ullTotalPageFile
    }

    /// The available memory (with page file) size for the process or computer, whichever is smaller.
    pub fn available_page_file(&self) -> u64 {
        self.0.ullAvailPageFile
    }

    /// The size of the total user-mode address space, in bytes.
    pub fn total_virtual(&self) -> u64 {
        self.0.ullTotalVirtual
    }

    /// The size of the available user-mode address space (unreserved and uncomitted), in bytes.
    pub fn available_virtual(&self) -> u64 {
        self.0.ullAvailVirtual
    }
}

impl std::fmt::Debug for MemoryStatusEx {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("MemoryStatusEx")
            .field("memory_load", &self.memory_load())
            .field("total_physical", &self.total_physical())
            .field("available_physical", &self.available_physical())
            .field("total_page_file", &self.total_page_file())
            .field("available_page_file", &self.available_page_file())
            .field("total_virtual", &self.total_virtual())
            .field("available_virtual", &self.available_virtual())
            .finish()
    }
}

/// A wrapper for `GlobalMemoryStatusEx`.
///
/// See `https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex`.
fn global_memory_status_ex() -> Result<MemoryStatusEx, Error> {
    const MEMORY_STATUS_EX_SIZE: u32 = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

    let mut memory_status_ex: MaybeUninit<MEMORYSTATUSEX> = unsafe {
        let mut memory_status_ex: MaybeUninit<MEMORYSTATUSEX> = MaybeUninit::uninit();
        std::ptr::addr_of_mut!((*memory_status_ex.as_mut_ptr()).dwLength)
            .write(MEMORY_STATUS_EX_SIZE);
        memory_status_ex
    };
    let code = unsafe { GlobalMemoryStatusEx(memory_status_ex.as_mut_ptr().cast()) };
    if code == 0 {
        return Err(Error::Io(std::io::Error::last_os_error()));
    }
    Ok(MemoryStatusEx(unsafe { memory_status_ex.assume_init() }))
}

/*
/// A wrapper for `GetLastError`.
///
/// See https://docs.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}
*/

struct PerformanceInformation(PERFORMANCE_INFORMATION);

impl PerformanceInformation {
    /// The number of pages currently committed by the system.
    ///
    /// Note that committing pages (using VirtualAlloc with MEM_COMMIT) changes this value immediately;
    /// however, the physical memory is not charged until the pages are accessed.
    pub fn commit_total(&self) -> usize {
        self.0.CommitTotal
    }

    /// The current maximum number of pages that can be committed by the system without extending the paging file(s).
    ///
    /// This number can change if memory is added or deleted,
    /// or if pagefiles have grown, shrunk, or been added.
    /// If the paging file can be extended, this is a soft limit.
    pub fn commit_limit(&self) -> usize {
        self.0.CommitLimit
    }

    /// The amount of actual physical memory, in pages.
    pub fn physical_total(&self) -> usize {
        self.0.PhysicalTotal
    }

    /// The amount of actual physical memory, in bytes.
    pub fn physical_total_bytes(&self) -> Option<usize> {
        self.physical_total().checked_mul(self.page_size())
    }

    /// The amount of physical memory currently available, in pages.
    ///
    /// This is the amount of physical memory that can be immediately
    /// reused without having to write its contents to disk first.
    /// It is the sum of the size of the standby, free, and zero lists.
    pub fn physical_available(&self) -> usize {
        self.0.PhysicalAvailable
    }

    /// The amount of physical memory currently available, in bytes.
    ///
    /// This is the amount of physical memory that can be immediately
    /// reused without having to write its contents to disk first.
    /// It is the sum of the size of the standby, free, and zero lists.
    pub fn physical_available_bytes(&self) -> Option<usize> {
        self.physical_available().checked_mul(self.page_size())
    }

    /// The size of a page, in bytes.
    pub fn page_size(&self) -> usize {
        self.0.PageSize
    }
}

impl std::fmt::Debug for PerformanceInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PerformanceInformation")
            .field("commit_total", &self.commit_total())
            .field("commit_limit", &self.commit_limit())
            .field("physical_total", &self.physical_total())
            .field("physical_total_bytes", &self.physical_total_bytes())
            .field("physical_available", &self.physical_available())
            .field("physical_available_bytes", &self.physical_available_bytes())
            .field("page_size", &self.page_size())
            .finish()
    }
}

fn get_performance_info() -> Result<PerformanceInformation, Error> {
    const SIZE: u32 = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;

    let mut performance_info = MaybeUninit::uninit();
    let result = unsafe { GetPerformanceInfo(performance_info.as_mut_ptr(), SIZE) };
    if result == FALSE {
        return Err(Error::Io(std::io::Error::last_os_error()));
    }
    let performance_info = unsafe { performance_info.assume_init() };

    Ok(PerformanceInformation(performance_info))
}

struct EnumPageFileInformation(*mut ENUM_PAGE_FILE_INFORMATION);

impl EnumPageFileInformation {
    /// The total size of the pagefile, in pages.
    fn total_size(&self) -> usize {
        unsafe { self.0.as_ref_unchecked() }.TotalSize
    }

    /// The current pagefile usage, in pages.
    fn total_in_use(&self) -> usize {
        unsafe { self.0.as_ref_unchecked() }.TotalInUse
    }
}

impl std::fmt::Debug for EnumPageFileInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("EnumPageFileInformation")
            .field("total_size", &self.total_size())
            .field("total_in_use", &self.total_in_use())
            .finish()
    }
}

fn enum_page_files<F, D>(user_data: D, func: F) -> Result<D, Error>
where
    F: FnMut(&mut D, EnumPageFileInformation, &WCStr) -> bool,
{
    struct Context<F, D> {
        user_data: D,
        func: F,
        panic_data: Option<Box<dyn Any + Send + 'static>>,
    }

    unsafe extern "system" fn callback<F, D>(
        context: *mut c_void,
        page_file_info: *mut ENUM_PAGE_FILE_INFORMATION,
        page_file_name: PCWSTR,
    ) -> BOOL
    where
        F: FnMut(&mut D, EnumPageFileInformation, &WCStr) -> bool,
    {
        let context = unsafe { context.cast::<Context<F, D>>().as_mut_unchecked() };
        let page_file_info = EnumPageFileInformation(page_file_info);
        let page_file_name = unsafe { WCStr::from_ptr(page_file_name) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (context.func)(&mut context.user_data, page_file_info, page_file_name)
        }));
        match result {
            Ok(true) => TRUE,
            Ok(false) => FALSE,
            Err(panic_data) => {
                // Store the panic and fail-fast.
                // We resume the panic outside the callback.
                context.panic_data = Some(panic_data);
                FALSE
            }
        }
    }

    let mut context = Context {
        func,
        user_data,
        panic_data: None,
    };

    let result = unsafe {
        let context_ptr = (&mut context as *mut Context<F, D>).cast::<c_void>();
        EnumPageFilesW(Some(callback::<F, D>), context_ptr)
    };
    // Panics are more important than errors.
    if let Some(panic_data) = context.panic_data {
        std::panic::resume_unwind(panic_data);
    }
    if result == FALSE {
        return Err(Error::Io(std::io::Error::last_os_error()));
    }

    Ok(context.user_data)
}

#[derive(Debug, PartialEq, Eq)]
enum LogicalProcessorRelationship {
    ProcessorCore,
}

impl TryFrom<LOGICAL_PROCESSOR_RELATIONSHIP> for LogicalProcessorRelationship {
    type Error = LOGICAL_PROCESSOR_RELATIONSHIP;

    fn try_from(value: LOGICAL_PROCESSOR_RELATIONSHIP) -> Result<Self, Self::Error> {
        #[expect(
            non_upper_case_globals,
            reason = "RelationProcessorCore is from windows-sys and we have no control over it."
        )]
        match value {
            RelationProcessorCore => Ok(Self::ProcessorCore),
            _ => Err(value),
        }
    }
}

#[derive(Clone)]
struct SystemLogicalProcessorInformationEx<'a>(&'a SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX);

impl SystemLogicalProcessorInformationEx<'_> {
    fn relationship(&self) -> Option<LogicalProcessorRelationship> {
        LogicalProcessorRelationship::try_from(self.0.Relationship).ok()
    }

    fn size(&self) -> u32 {
        self.0.Size
    }
}

impl std::fmt::Debug for SystemLogicalProcessorInformationEx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("SystemLogicalProcessorInformationEx")
            .field("relationship", &self.relationship())
            .field("size", &self.size())
            .finish()
    }
}

struct LogicalProcessorInformationExStore {
    buffer: Vec<u8>,
}

impl LogicalProcessorInformationExStore {
    fn cursor(&self) -> LogicalProcessorInformationExStoreCursor<'_> {
        LogicalProcessorInformationExStoreCursor::new(self)
    }
}

struct LogicalProcessorInformationExStoreCursor<'a> {
    store: &'a LogicalProcessorInformationExStore,
    current: Option<SystemLogicalProcessorInformationEx<'a>>,
    offset: usize,
}

impl<'a> LogicalProcessorInformationExStoreCursor<'a> {
    fn new(store: &'a LogicalProcessorInformationExStore) -> Self {
        let mut cursor = Self {
            store,
            current: None,
            offset: 0,
        };

        unsafe { cursor.read_current() };

        cursor
    }

    /// Safety
    /// `offset` must point at a valid SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX in `buffer`,
    /// or be at the end.
    unsafe fn read_current(&mut self) {
        self.current = if self.offset < self.store.buffer.len() {
            Some(SystemLogicalProcessorInformationEx(unsafe {
                self.store
                    .buffer
                    .as_ptr()
                    .add(self.offset)
                    .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>()
                    .as_ref()
                    .expect("Invalid ptr")
            }))
        } else {
            None
        };
    }

    fn get(&self) -> Option<SystemLogicalProcessorInformationEx<'a>> {
        self.current.clone()
    }

    fn advance(&mut self) -> bool {
        let current = match self.current.take() {
            Some(current) => current,
            None => return false,
        };
        self.offset += usize::try_from(current.size()).expect("Size greater than usize");

        unsafe {
            self.read_current();
        }
        if self.current.is_none() {
            return false;
        }

        true
    }
}

fn get_logical_processor_information_ex() -> Result<LogicalProcessorInformationExStore, Error> {
    let mut buffer_len_bytes = 0;
    let mut buffer: Vec<u8> = Vec::new();
    let result = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            buffer
                .as_mut_ptr()
                .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
            &mut buffer_len_bytes,
        )
    };
    if result == TRUE {
        return Ok(LogicalProcessorInformationExStore { buffer });
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32) {
        return Err(Error::Io(error));
    }

    let buffer_len_usize =
        usize::try_from(buffer_len_bytes).expect("buffer_len larger than a usize");
    buffer.reserve(buffer_len_usize);
    let result = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            buffer
                .as_mut_ptr()
                .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(),
            &mut buffer_len_bytes,
        )
    };
    if result == FALSE {
        return Err(Error::Io(std::io::Error::last_os_error()));
    }
    let buffer_len_usize =
        usize::try_from(buffer_len_bytes).expect("buffer_len larger than a usize");
    unsafe { buffer.set_len(buffer_len_usize) };

    Ok(LogicalProcessorInformationExStore { buffer })
}

#[repr(transparent)]
struct ProcessorPowerInformation(PROCESSOR_POWER_INFORMATION);

impl ProcessorPowerInformation {
    /// Get the processor number.
    fn number(&self) -> u32 {
        self.0.Number
    }

    /// Get the max processor speed.
    fn max_mhz(&self) -> u32 {
        self.0.MaxMhz
    }

    /// Get the current processor speed.
    fn current_mhz(&self) -> u32 {
        self.0.CurrentMhz
    }

    /// Get the max processor speed.
    fn limit_mhz(&self) -> u32 {
        self.0.MhzLimit
    }
}

impl std::fmt::Debug for ProcessorPowerInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ProcessorPowerInformation")
            .field("number", &self.number())
            .field("max_mhz", &self.max_mhz())
            .field("current_mhz", &self.current_mhz())
            .field("limit_mhz", &self.limit_mhz())
            .finish()
    }
}

#[allow(dead_code)]
fn call_nt_power_information(
    processor_count: usize,
) -> Result<Vec<ProcessorPowerInformation>, Error> {
    let mut info: Vec<ProcessorPowerInformation> = Vec::with_capacity(processor_count);
    let info_len_bytes = processor_count * std::mem::size_of::<PROCESSOR_POWER_INFORMATION>();
    let result = unsafe {
        CallNtPowerInformation(
            ProcessorInformation,
            std::ptr::null(),
            0,
            info.as_mut_ptr().cast(),
            u32::try_from(info_len_bytes).expect("Core count too large"),
        )
    };

    if result != STATUS_SUCCESS {
        let error = unsafe { RtlNtStatusToDosError(result) };
        return Err(Error::Io(std::io::Error::from_raw_os_error(error as i32)));
    }
    unsafe {
        info.set_len(processor_count);
    }

    Ok(info)
}

struct PdhQuery {
    query_handle: PDH_HQUERY,
    counters: Vec<PdhCounter>,
}

impl PdhQuery {
    fn new() -> Result<Self, Error> {
        let mut handle: MaybeUninit<PDH_HQUERY> = MaybeUninit::zeroed();
        let result = unsafe { PdhOpenQueryW(std::ptr::null(), 0, handle.as_mut_ptr().cast()) };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }
        let query_handle = unsafe { handle.assume_init() };

        Ok(Self {
            query_handle,
            counters: Vec::new(),
        })
    }

    fn add_counter(&mut self, query: &str) -> Result<usize, Error> {
        let query_str_wide: Vec<u16> = query.encode_utf16().chain(Some(0)).collect();
        let mut handle: MaybeUninit<PDH_HCOUNTER> = MaybeUninit::zeroed();
        let result = unsafe {
            PdhAddEnglishCounterW(
                self.query_handle,
                query_str_wide.as_ptr(),
                0,
                handle.as_mut_ptr(),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }

        let handle = unsafe { handle.assume_init() };
        let pdh_counter = PdhCounter { handle };
        let counter_index = self.counters.len();
        let _pdh_counter = self.counters.push_mut(pdh_counter);

        Ok(counter_index)
    }

    fn get_counter(&self, index: usize) -> Option<&PdhCounter> {
        self.counters.get(index)
    }

    fn collect(&self) -> Result<(), Error> {
        let result = unsafe { PdhCollectQueryData(self.query_handle) };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }

        Ok(())
    }
}

impl std::fmt::Debug for PdhQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PdhQuery").finish()
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        let _result = unsafe { PdhCloseQuery(self.query_handle) };
    }
}

// Safety: PDH_HQUERY has no thread-local state.
unsafe impl Send for PdhQuery {}

#[repr(transparent)]
struct PdhRawItem(PDH_RAW_COUNTER_ITEM_W);

impl PdhRawItem {
    fn name(&self) -> &WCStr {
        unsafe { WCStr::from_ptr(self.0.szName) }
    }

    fn ensure_valid(&self) -> Result<(), Error> {
        if self.0.RawValue.CStatus != PDH_CSTATUS_VALID_DATA
            && self.0.RawValue.CStatus != PDH_CSTATUS_NEW_DATA
        {
            return Err(Error::Io(std::io::Error::from_raw_os_error(
                self.0.RawValue.CStatus as i32,
            )));
        }

        Ok(())
    }

    fn first_value(&self) -> Result<i64, Error> {
        self.ensure_valid()?;

        Ok(self.0.RawValue.FirstValue)
    }

    fn second_value(&self) -> Result<i64, Error> {
        self.ensure_valid()?;

        Ok(self.0.RawValue.SecondValue)
    }
}

impl std::fmt::Debug for PdhRawItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PdhRawItem")
            .field("name", &self.name())
            .field("first_value", &self.first_value())
            .field("second_value", &self.second_value())
            .finish()
    }
}

#[repr(transparent)]
struct PdhFormattedCounterValue(PDH_FMT_COUNTERVALUE);

impl PdhFormattedCounterValue {
    fn ensure_valid(&self) -> Result<(), Error> {
        if self.0.CStatus != PDH_CSTATUS_VALID_DATA && self.0.CStatus != PDH_CSTATUS_NEW_DATA {
            return Err(Error::Io(std::io::Error::from_raw_os_error(
                self.0.CStatus as i32,
            )));
        }

        Ok(())
    }

    fn double(&self) -> Result<f64, Error> {
        self.ensure_valid()?;

        // Safety: All bit-patterns of double are valid.
        Ok(unsafe { self.0.Anonymous.doubleValue })
    }
}

impl std::fmt::Debug for PdhFormattedCounterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PdhFormattedCounterValue")
            .field("double", &self.double())
            .finish()
    }
}

#[repr(transparent)]
struct PdhFmtValueItem(PDH_FMT_COUNTERVALUE_ITEM_W);

impl PdhFmtValueItem {
    fn name(&self) -> &WCStr {
        unsafe { WCStr::from_ptr(self.0.szName) }
    }

    fn value(&self) -> &PdhFormattedCounterValue {
        unsafe {
            (&self.0.FmtValue as *const PDH_FMT_COUNTERVALUE)
                .cast::<PdhFormattedCounterValue>()
                .as_ref_unchecked()
        }
    }
}

impl std::fmt::Debug for PdhFmtValueItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PdhFmtValueItem")
            .field("name", &self.name())
            .field("value", &self.value())
            .finish()
    }
}

#[expect(dead_code)]
struct PdhItemArray<T> {
    buffer: Vec<u8>,
    item_count: usize,
    _data: PhantomData<T>,
}

#[expect(dead_code)]
impl<T> PdhItemArray<T> {
    fn len(&self) -> usize {
        self.item_count
    }

    #[expect(dead_code)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get(&self, index: usize) -> Option<&T> {
        if index >= self.item_count {
            return None;
        }

        let item: &T = unsafe {
            self.buffer
                .as_ptr()
                .cast::<T>()
                .add(index)
                .as_ref_unchecked()
        };

        Some(item)
    }
}

struct PdhCounter {
    handle: PDH_HCOUNTER,
}

impl PdhCounter {
    #[expect(dead_code)]
    fn get_raw_array(&self) -> Result<PdhItemArray<PdhRawItem>, Error> {
        let mut buffer_size: u32 = 0;
        let mut item_count: u32 = 0;
        let result = unsafe {
            PdhGetRawCounterArrayW(
                self.handle,
                &mut buffer_size,
                &mut item_count,
                std::ptr::null_mut(),
            )
        };
        match result {
            ERROR_SUCCESS => {
                return Ok(PdhItemArray {
                    buffer: Vec::new(),
                    item_count: 0,
                    _data: PhantomData,
                });
            }
            PDH_MORE_DATA => {}
            _ => return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32))),
        }

        let buffer_size_usize = usize::try_from(buffer_size).expect("buffer_size too large");
        let mut buffer: Vec<u8> = vec![0; buffer_size_usize];
        let result = unsafe {
            PdhGetRawCounterArrayW(
                self.handle,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr().cast(),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }

        Ok(PdhItemArray {
            buffer,
            item_count: usize::try_from(item_count).expect("item_count too large"),
            _data: PhantomData,
        })
    }

    #[expect(dead_code)]
    fn get_formatted_array(&self) -> Result<PdhItemArray<PdhFmtValueItem>, Error> {
        let format = PDH_FMT_DOUBLE;

        let mut buffer_size: u32 = 0;
        let mut item_count: u32 = 0;
        let result = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                format,
                &mut buffer_size,
                &mut item_count,
                std::ptr::null_mut(),
            )
        };
        match result {
            ERROR_SUCCESS => {
                return Ok(PdhItemArray {
                    buffer: Vec::new(),
                    item_count: 0,
                    _data: PhantomData,
                });
            }
            PDH_MORE_DATA => {}
            _ => return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32))),
        }

        let buffer_size_usize = usize::try_from(buffer_size).expect("buffer_size too large");
        let mut buffer: Vec<u8> = vec![0; buffer_size_usize];
        let result = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                format,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr().cast(),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }

        Ok(PdhItemArray {
            buffer,
            item_count: usize::try_from(item_count).expect("item_count too large"),
            _data: PhantomData,
        })
    }

    fn get_formatted_value(&self) -> Result<PdhFormattedCounterValue, Error> {
        let format = PDH_FMT_DOUBLE;
        let mut value: MaybeUninit<PdhFormattedCounterValue> = MaybeUninit::zeroed();
        let result = unsafe {
            PdhGetFormattedCounterValue(
                self.handle,
                format,
                std::ptr::null_mut(),
                value.as_mut_ptr().cast(),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(Error::Io(std::io::Error::from_raw_os_error(result as i32)));
        }

        Ok(unsafe { value.assume_init() })
    }
}

impl std::fmt::Debug for PdhCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("PdhCounter").finish()
    }
}

#[derive(Debug)]
struct PageFileStats {
    /// The total size of all page files, in pages.
    total: usize,

    /// The total usage of all page files, in pages.
    used: usize,
}

#[derive(Debug)]
struct PdhCounters {
    query: PdhQuery,
    cpu_frequency_counter_index: usize,
}

/// A cache for system data
#[derive(Debug)]
pub(crate) struct CacheContext {
    system_info: OnceCell<SystemInfo>,
    memory_status_ex: OnceCell<MemoryStatusEx>,
    os_version_info_ex: OnceCell<OsVersionInfoEx>,
    performance_info: OnceCell<PerformanceInformation>,
    page_file_stats: OnceCell<PageFileStats>,
    pdh_counters: OnceCell<Mutex<PdhCounters>>,
}

impl CacheContext {
    /// Make a new cache context
    pub fn new() -> Self {
        Self {
            system_info: OnceCell::new(),
            memory_status_ex: OnceCell::new(),
            os_version_info_ex: OnceCell::new(),
            performance_info: OnceCell::new(),
            page_file_stats: OnceCell::new(),
            pdh_counters: OnceCell::new(),
        }
    }

    fn get_system_info(&self) -> &SystemInfo {
        self.system_info.get_or_init(get_system_info)
    }

    fn get_memory_status_ex(&self) -> Result<&MemoryStatusEx, Error> {
        self.memory_status_ex
            .get_or_try_init(global_memory_status_ex)
    }

    fn get_os_version_info(&self) -> &OsVersionInfoEx {
        self.os_version_info_ex.get_or_init(rtl_get_version)
    }

    fn get_performance_info(&self) -> Result<&PerformanceInformation, Error> {
        self.performance_info.get_or_try_init(get_performance_info)
    }

    fn get_page_file_stats(&self) -> Result<&PageFileStats, Error> {
        self.page_file_stats.get_or_try_init(|| {
            enum_page_files(
                PageFileStats { total: 0, used: 0 },
                |stats, info, _file_name| {
                    stats.total += info.total_size();
                    stats.used += info.total_in_use();
                    true
                },
            )
        })
    }

    /// Start measuring.
    pub fn start(&self) -> Result<(), Error> {
        self.pdh_counters.get_or_try_init(|| {
            let mut query = PdhQuery::new()?;
            let cpu_frequency_counter_index =
                query.add_counter("\\Processor Information(_Total)\\Actual Frequency")?;
            query.collect()?;

            Ok::<_, Error>(Mutex::new(PdhCounters {
                query,
                cpu_frequency_counter_index,
            }))
        })?;

        Ok(())
    }

    /// End measuring.
    pub fn end(&self) -> Result<(), Error> {
        let pdh_counters = self
            .pdh_counters
            .get()
            .ok_or(Error::Generic("Need to call `start`"))?;
        {
            let pdh_counters = pdh_counters
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            pdh_counters.query.collect()?;
        }

        Ok(())
    }

    /// Get the time the system was booted.
    pub fn get_boot_time(&self) -> Result<SystemTime, Error> {
        Ok(SystemTime::now() - self.get_uptime()?)
    }

    /// Get the uptime.
    pub fn get_uptime(&self) -> Result<Duration, Error> {
        Ok(get_tick_count_64())
    }

    /// Get the hostname
    pub fn get_hostname(&self) -> Result<String, Error> {
        get_computer_name(ComputerNameFormat::PhysicalDnsHostname)?
            .into_string()
            .map_err(|value| Error::InvalidUtf8OsString { value })
    }

    /// Get the architecture.
    pub fn get_architecture(&self) -> Result<Option<Arch>, Error> {
        let system_info = self.get_system_info();
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
    pub fn get_system_name(&self) -> Result<Option<String>, Error> {
        let os_version_info = self.get_os_version_info();

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

    /// Get the os version.
    pub fn get_system_version(&self) -> Result<String, Error> {
        let os_version_info = self.get_os_version_info();

        let major_version = os_version_info.major_version();
        let minor_version = os_version_info.minor_version();
        let build_number = os_version_info.build_number();

        Ok(format!("{major_version}.{minor_version}.{build_number}"))
    }

    /// Get the total amount of memory in the computer, in bytes.
    pub fn get_total_memory(&self) -> Result<u64, Error> {
        let memory_info_ex = self.get_memory_status_ex()?;
        Ok(memory_info_ex.total_physical())
    }

    /// Get the available amount of memory in the computer, in bytes.
    pub fn get_available_memory(&self) -> Result<u64, Error> {
        let memory_info_ex = self.get_memory_status_ex()?;
        Ok(memory_info_ex.available_physical())
    }

    /// Get the total amount of swap in the computer, in bytes.
    pub fn get_total_swap(&self) -> Result<u64, Error> {
        let performance_info = self.get_performance_info()?;
        let page_file_stats = self.get_page_file_stats()?;

        let page_size = performance_info.page_size();

        page_file_stats
            .total
            .checked_mul(page_size)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(Error::Generic("Failed to calculate total swap size"))
    }

    /// Get the available amount of swap in the computer, in bytes.
    pub fn get_available_swap(&self) -> Result<u64, Error> {
        let performance_info = self.get_performance_info()?;
        let page_file_stats = self.get_page_file_stats()?;

        let page_size = performance_info.page_size();

        page_file_stats
            .total
            .checked_sub(page_file_stats.used)
            .and_then(|value| value.checked_mul(page_size))
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(Error::Generic("Failed to calculate available swap size"))
    }

    /// Get the number of logical cpu cores.
    pub fn count_logical_cpus(&self) -> Result<usize, Error> {
        Ok(get_active_processor_count(u16::MAX)?
            .try_into()
            .expect("Windows is 16 bit"))
    }

    /// Get the number of physical cpus cores.
    pub fn count_physical_cpus(&self) -> Result<usize, Error> {
        let store = get_logical_processor_information_ex()?;
        let mut cursor = store.cursor();

        let mut physical_core_count = 0;
        while let Some(info) = cursor.get() {
            if info.relationship() == Some(LogicalProcessorRelationship::ProcessorCore) {
                physical_core_count += 1;
            }

            cursor.advance();
        }

        Ok(physical_core_count)
    }

    /// Get the cpu frequency.
    pub fn get_cpu_frequency(&self) -> Result<u64, Error> {
        // Inspired by: https://stackoverflow.com/questions/61802420/unable-to-get-current-cpu-frequency-in-powershell-or-python

        let pdh_counters = self
            .pdh_counters
            .get()
            .ok_or(Error::Generic("Need to call `start`"))?;
        let value = {
            let pdh_counters = pdh_counters
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let counter = pdh_counters
                .query
                .get_counter(pdh_counters.cpu_frequency_counter_index)
                .expect("Missing cpu frequency counter");
            counter.get_formatted_value()?.double()?
        };

        // The returned value is in MHz.
        // Convert to Hz.
        let value = value * 1_000_000.0;

        Ok(value as u64)
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
    fn get_tick_count_64_does_not_block() {
        let start = Instant::now();
        let _boot_time = CacheContext::new().get_boot_time();
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_computer_name_does_not_block() {
        let start = Instant::now();
        let _hostname = get_computer_name(ComputerNameFormat::PhysicalDnsHostname);
        assert!(start.elapsed() < BLOCKING_DURATION);
    }

    #[test]
    fn get_system_info_does_not_block() {
        let start = Instant::now();
        let _system_info = get_system_info();
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);
    }

    #[test]
    fn rtl_get_version_ex_works() {
        let start = Instant::now();
        let version = rtl_get_version();
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        dbg!(version);
    }

    #[test]
    fn global_memory_status_ex_works() {
        let start = Instant::now();
        let memory_status = global_memory_status_ex().expect("Failed to get global memory status");
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);

        dbg!(memory_status);
    }

    #[test]
    fn get_active_processor_count_does_not_block() {
        let start = Instant::now();
        let _logical_cpus = get_active_processor_count(u16::MAX);
        let elapsed = start.elapsed();
        assert!(elapsed < BLOCKING_DURATION);
    }

    #[test]
    fn get_performance_info_works() {
        let start = Instant::now();
        let _performance_info =
            get_performance_info().expect("get_performance_info returned an error");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );
    }

    #[test]
    fn enum_page_files_works() {
        let start = Instant::now();
        let _page_file_info = enum_page_files((0, 0), |(total, used), info, _name| {
            *total = info.total_size();
            *used = info.total_in_use();
            true
        })
        .expect("enum_page_files returned an error");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );
    }

    #[test]
    fn get_logical_processor_information_ex_works() {
        let start = Instant::now();
        let _info = get_logical_processor_information_ex()
            .expect("get_logical_processor_information_ex returned an error");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );
    }

    #[test]
    fn call_nt_power_information_works() {
        let start = Instant::now();
        let system_info = get_system_info();
        let info = call_nt_power_information(
            usize::try_from(system_info.number_of_processors()).expect("Core count too large"),
        )
        .expect("call_nt_power_information returned an error");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );
        dbg!(info);
    }

    #[test]
    fn pdh_query_works() {
        let start = Instant::now();
        let mut query = PdhQuery::new().expect("Failed to create query");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );

        // Adding a counter blocks for about ~150 ms.
        let start = Instant::now();
        let counter_index = query
            .add_counter("\\Processor Information(_Total)\\Actual Frequency")
            .expect("Failed to create counter");
        let counter = query.get_counter(counter_index).expect("Missing counter");
        let elapsed = start.elapsed();
        assert!(
            elapsed > BLOCKING_DURATION,
            "Operation did not block, took {elapsed:?}"
        );

        let start = Instant::now();
        query.collect().expect("Failed to collect data");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );

        std::thread::sleep(std::time::Duration::from_secs(1));

        let start = Instant::now();
        query.collect().expect("Failed to collect data");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );

        let start = Instant::now();
        let item = counter.get_formatted_value().expect("Failed to get value");
        let elapsed = start.elapsed();
        assert!(
            elapsed < BLOCKING_DURATION,
            "Blocked, operation took {elapsed:?}"
        );
        dbg!(item);
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
