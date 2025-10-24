//! UEFI File System Protocol

use super::{EFI_GUID, EFI_STATUS};
use core::ffi::c_void;

/// Simple File System Protocol GUID: 964E5B22-6459-11D2-8E39-00A0C969723B
pub const SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: EFI_GUID = EFI_GUID {
    data1: 0x964e5b22,
    data2: 0x6459,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// File open modes
pub const EFI_FILE_MODE_READ: u64 = 0x0000000000000001;
pub const EFI_FILE_MODE_WRITE: u64 = 0x0000000000000002;
pub const EFI_FILE_MODE_CREATE: u64 = 0x8000000000000000;

/// File attributes
pub const EFI_FILE_READ_ONLY: u64 = 0x0000000000000001;
pub const EFI_FILE_HIDDEN: u64 = 0x0000000000000002;
pub const EFI_FILE_SYSTEM: u64 = 0x0000000000000004;
pub const EFI_FILE_RESERVED: u64 = 0x0000000000000008;
pub const EFI_FILE_DIRECTORY: u64 = 0x0000000000000010;
pub const EFI_FILE_ARCHIVE: u64 = 0x0000000000000020;

/// UEFI Simple File System Protocol
#[repr(C)]
pub struct EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    pub revision: u64,
    pub open_volume: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_FILE_SYSTEM_PROTOCOL,
        root: *mut *mut EFI_FILE_PROTOCOL,
    ) -> EFI_STATUS,
}

/// UEFI File Protocol
#[repr(C)]
pub struct EFI_FILE_PROTOCOL {
    pub revision: u64,
    
    pub open: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        new_handle: *mut *mut EFI_FILE_PROTOCOL,
        file_name: *const u16,
        open_mode: u64,
        attributes: u64,
    ) -> EFI_STATUS,
    
    pub close: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
    ) -> EFI_STATUS,
    
    pub delete: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
    ) -> EFI_STATUS,
    
    pub read: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> EFI_STATUS,
    
    pub write: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        buffer_size: *mut usize,
        buffer: *const u8,
    ) -> EFI_STATUS,
    
    pub get_position: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        position: *mut u64,
    ) -> EFI_STATUS,
    
    pub set_position: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        position: u64,
    ) -> EFI_STATUS,
    
    pub get_info: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        information_type: *const EFI_GUID,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> EFI_STATUS,
    
    pub set_info: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
        information_type: *const EFI_GUID,
        buffer_size: usize,
        buffer: *const u8,
    ) -> EFI_STATUS,
    
    pub flush: extern "efiapi" fn(
        this: *mut EFI_FILE_PROTOCOL,
    ) -> EFI_STATUS,
}

/// File Info GUID
pub const EFI_FILE_INFO_GUID: EFI_GUID = EFI_GUID {
    data1: 0x09576e92,
    data2: 0x6d3f,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

/// UEFI File Info structure
#[repr(C)]
pub struct EFI_FILE_INFO {
    pub size: u64,
    pub file_size: u64,
    pub physical_size: u64,
    pub create_time: EFI_TIME,
    pub last_access_time: EFI_TIME,
    pub modification_time: EFI_TIME,
    pub attribute: u64,
    // file_name: [u16] - variable length
}

/// UEFI Time structure
#[repr(C)]
pub struct EFI_TIME {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub pad1: u8,
    pub nanosecond: u32,
    pub time_zone: i16,
    pub daylight: u8,
    pub pad2: u8,
}
