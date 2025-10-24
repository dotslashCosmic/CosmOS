//! Custom UEFI type definitions and structures

use core::ffi::c_void;

// Re-export submodules
pub mod console;
pub mod file;
pub mod memory;
pub mod boot;

/// UEFI status code type
pub type EFI_STATUS = usize;

/// UEFI handle type (opaque pointer)
pub type EFI_HANDLE = *mut c_void;

/// Success status code
pub const EFI_SUCCESS: EFI_STATUS = 0;

/// Error status codes
pub const EFI_LOAD_ERROR: EFI_STATUS = 1;
pub const EFI_INVALID_PARAMETER: EFI_STATUS = 2;
pub const EFI_UNSUPPORTED: EFI_STATUS = 3;
pub const EFI_BAD_BUFFER_SIZE: EFI_STATUS = 4;
pub const EFI_BUFFER_TOO_SMALL: EFI_STATUS = 5;
pub const EFI_NOT_READY: EFI_STATUS = 6;
pub const EFI_DEVICE_ERROR: EFI_STATUS = 7;
pub const EFI_WRITE_PROTECTED: EFI_STATUS = 8;
pub const EFI_OUT_OF_RESOURCES: EFI_STATUS = 9;
pub const EFI_NOT_FOUND: EFI_STATUS = 14;

/// UEFI GUID
#[repr(C)]
#[derive(Copy, Clone)]
pub struct EFI_GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

/// UEFI table header
#[repr(C)]
pub struct EFI_TABLE_HEADER {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

/// UEFI Configuration Table
#[repr(C)]
pub struct EFI_CONFIGURATION_TABLE {
    pub vendor_guid: EFI_GUID,
    pub vendor_table: *mut c_void,
}

/// UEFI Runtime Services
#[repr(C)]
pub struct EFI_RUNTIME_SERVICES {
    pub hdr: EFI_TABLE_HEADER,
    // Not needed for bootloader
}

/// UEFI Boot Services Table
#[repr(C)]
pub struct EFI_BOOT_SERVICES {
    pub hdr: EFI_TABLE_HEADER,
    
    // Task Priority Services (padding)
    _raise_tpl: usize,
    _restore_tpl: usize,
    
    // Memory Services
    pub allocate_pages: extern "efiapi" fn(
        alloc_type: u32,
        memory_type: u32,
        pages: usize,
        memory: *mut u64,
    ) -> EFI_STATUS,
    
    pub free_pages: extern "efiapi" fn(memory: u64, pages: usize) -> EFI_STATUS,
    
    pub get_memory_map: extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut u8,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EFI_STATUS,
    
    pub allocate_pool: extern "efiapi" fn(
        pool_type: u32,
        size: usize,
        buffer: *mut *mut u8,
    ) -> EFI_STATUS,
    
    pub free_pool: extern "efiapi" fn(buffer: *mut u8) -> EFI_STATUS,
    
    // Event & Timer Services, 8 function pointers
    _create_event: usize,
    _set_timer: usize,
    _wait_for_event: usize,
    _signal_event: usize,
    _close_event: usize,
    _check_event: usize,
    
    // Protocol Handler Services, 9 function pointers
    _install_protocol_interface: usize,
    _reinstall_protocol_interface: usize,
    _uninstall_protocol_interface: usize,
    _handle_protocol: usize,
    _reserved: usize,
    _register_protocol_notify: usize,
    
    pub locate_handle: extern "efiapi" fn(
        search_type: u32,
        protocol: *const EFI_GUID,
        search_key: *mut c_void,
        buffer_size: *mut usize,
        buffer: *mut EFI_HANDLE,
    ) -> EFI_STATUS,
    
    _locate_device_path: usize,
    _install_configuration_table: usize,
    
    // Image Services, 4 function pointers
    _load_image: usize,
    _start_image: usize,
    _exit: usize,
    _unload_image: usize,
    
    pub exit_boot_services: extern "efiapi" fn(
        image_handle: EFI_HANDLE,
        map_key: usize,
    ) -> EFI_STATUS,
    
    // Miscellaneous Services, 6 function pointers
    _get_next_monotonic_count: usize,
    _stall: usize,
    _set_watchdog_timer: usize,
    
    // DriverSupport Services, 2 function pointers
    _connect_controller: usize,
    _disconnect_controller: usize,
    
    // Open and Close Protocol Services, 3 function pointers
    _open_protocol: usize,
    _close_protocol: usize,
    _open_protocol_information: usize,
    
    // Library Services, 2 function pointers
    _protocols_per_handle: usize,
    _locate_handle_buffer: usize,
    
    pub locate_protocol: extern "efiapi" fn(
        protocol: *const EFI_GUID,
        registration: *mut c_void,
        interface: *mut *mut c_void,
    ) -> EFI_STATUS,
    
    // More services
    _install_multiple_protocol_interfaces: usize,
    _uninstall_multiple_protocol_interfaces: usize,
    
    // CRC Services
    _calculate_crc32: usize,
    
    // Miscellaneous Services, 3 function pointers
    _copy_mem: usize,
    _set_mem: usize,
    _create_event_ex: usize,
}

/// UEFI Simple Text Input Protocol, unused
#[repr(C)]
pub struct EFI_SIMPLE_TEXT_INPUT_PROTOCOL {
    _reset: usize,
    _read_key_stroke: usize,
    _wait_for_key: *mut c_void,
}

/// UEFI System Table
#[repr(C)]
pub struct EFI_SYSTEM_TABLE {
    pub hdr: EFI_TABLE_HEADER,
    pub firmware_vendor: *const u16,
    pub firmware_revision: u32,
    pub console_in_handle: EFI_HANDLE,
    pub con_in: *mut EFI_SIMPLE_TEXT_INPUT_PROTOCOL,
    pub console_out_handle: EFI_HANDLE,
    pub con_out: *mut console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    pub standard_error_handle: EFI_HANDLE,
    pub std_err: *mut console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    pub runtime_services: *mut EFI_RUNTIME_SERVICES,
    pub boot_services: *mut EFI_BOOT_SERVICES,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut EFI_CONFIGURATION_TABLE,
}
