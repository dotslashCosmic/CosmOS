//! UEFI Memory Management

/// UEFI Memory Types
pub const EFI_RESERVED_MEMORY_TYPE: u32 = 0;
pub const EFI_LOADER_CODE: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;
pub const EFI_BOOT_SERVICES_CODE: u32 = 3;
pub const EFI_BOOT_SERVICES_DATA: u32 = 4;
pub const EFI_RUNTIME_SERVICES_CODE: u32 = 5;
pub const EFI_RUNTIME_SERVICES_DATA: u32 = 6;
pub const EFI_CONVENTIONAL_MEMORY: u32 = 7;
pub const EFI_UNUSABLE_MEMORY: u32 = 8;
pub const EFI_ACPI_RECLAIM_MEMORY: u32 = 9;
pub const EFI_ACPI_MEMORY_NVS: u32 = 10;
pub const EFI_MEMORY_MAPPED_IO: u32 = 11;
pub const EFI_MEMORY_MAPPED_IO_PORT_SPACE: u32 = 12;
pub const EFI_PAL_CODE: u32 = 13;
pub const EFI_PERSISTENT_MEMORY: u32 = 14;

/// UEFI Memory Descriptor
#[repr(C)]
#[derive(Copy, Clone)]
pub struct EFI_MEMORY_DESCRIPTOR {
    pub memory_type: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

/// E820 Memory Map Entry
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct E820Entry {
    pub base: u64,
    pub length: u64,
    pub entry_type: u32,
    pub acpi: u32,
}

/// E820 Memory Types
pub const E820_USABLE: u32 = 1;
pub const E820_RESERVED: u32 = 2;
pub const E820_ACPI_RECLAIMABLE: u32 = 3;
pub const E820_ACPI_NVS: u32 = 4;
pub const E820_BAD_MEMORY: u32 = 5;

/// Memory allocation types
pub const ALLOCATE_ANY_PAGES: u32 = 0;
pub const ALLOCATE_MAX_ADDRESS: u32 = 1;
pub const ALLOCATE_ADDRESS: u32 = 2;
