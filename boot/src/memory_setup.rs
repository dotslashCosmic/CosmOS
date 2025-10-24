//! Memory Setup Module

use crate::uefi::{
    EFI_BOOT_SERVICES, EFI_SUCCESS, EFI_BUFFER_TOO_SMALL,
    console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    memory::{
        EFI_MEMORY_DESCRIPTOR, E820Entry,
        EFI_CONVENTIONAL_MEMORY, EFI_LOADER_CODE, EFI_LOADER_DATA,
        EFI_BOOT_SERVICES_CODE, EFI_BOOT_SERVICES_DATA,
        EFI_ACPI_RECLAIM_MEMORY, EFI_ACPI_MEMORY_NVS,
        E820_USABLE, E820_RESERVED, E820_ACPI_RECLAIMABLE, E820_ACPI_NVS,
    },
};
use crate::{println, error};

/// Memory map information returned from UEFI
pub struct MemoryMapInfo {
    pub map_key: usize,
    pub descriptor_size: usize,
    pub descriptor_count: usize,
}

/// Static buffer for memory map
static mut MEMORY_MAP_BUFFER: [u8; 8192] = [0; 8192];

/// Static buffer for E820 entries, 128 entries
static mut E820_BUFFER: [E820Entry; 128] = [E820Entry {
    base: 0,
    length: 0,
    entry_type: 0,
    acpi: 0,
}; 128];

/// Get UEFI memory map
pub unsafe fn get_uefi_memory_map(
    boot_services: *mut EFI_BOOT_SERVICES,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> MemoryMapInfo {
    let mut map_size = MEMORY_MAP_BUFFER.len();
    let mut map_key: usize = 0;
    let mut descriptor_size: usize = 0;
    let mut descriptor_version: u32 = 0;
    
    // Call get_memory_map
    let status = ((*boot_services).get_memory_map)(
        &mut map_size,
        MEMORY_MAP_BUFFER.as_mut_ptr(),
        &mut map_key,
        &mut descriptor_size,
        &mut descriptor_version,
    );
    
    if status != EFI_SUCCESS {
        if status == EFI_BUFFER_TOO_SMALL {
            error::display_error_and_halt(
                console,
                "Memory map buffer too small - Increase MEMORY_MAP_BUFFER size",
                status,
            );
        } else {
            error::display_error_and_halt(
                console,
                "Failed to retrieve UEFI memory map",
                status,
            );
        }
    }
    
    if descriptor_size == 0 {
        error::display_simple_error_and_halt(
            console,
            "Invalid memory map - descriptor size is zero",
        );
    }
    
    let descriptor_count = map_size / descriptor_size;
    
    if descriptor_count == 0 {
        error::display_simple_error_and_halt(
            console,
            "Invalid memory map - no memory descriptors found",
        );
    }
    
    MemoryMapInfo {
        map_key,
        descriptor_size,
        descriptor_count,
    }
}

/// Convert UEFI memory type to E820 type
fn uefi_type_to_e820(uefi_type: u32) -> u32 {
    match uefi_type {
        EFI_CONVENTIONAL_MEMORY => E820_USABLE,
        EFI_LOADER_CODE => E820_USABLE,
        EFI_LOADER_DATA => E820_USABLE,
        EFI_BOOT_SERVICES_CODE => E820_USABLE,
        EFI_BOOT_SERVICES_DATA => E820_USABLE,
        EFI_ACPI_RECLAIM_MEMORY => E820_ACPI_RECLAIMABLE,
        EFI_ACPI_MEMORY_NVS => E820_ACPI_NVS,
        _ => E820_RESERVED,
    }
}

/// Convert UEFI memory map to E820 format
pub unsafe fn convert_uefi_to_e820(
    descriptor_size: usize,
    descriptor_count: usize,
) -> usize {
    let mut e820_count = 0;
    
    for i in 0..descriptor_count {
        if e820_count >= E820_BUFFER.len() {
            break; // Buffer full
        }
        
        // Get pointer to current descriptor
        let desc_ptr = MEMORY_MAP_BUFFER.as_ptr().add(i * descriptor_size) as *const EFI_MEMORY_DESCRIPTOR;
        let desc = &*desc_ptr;
        
        // Convert UEFI type to E820 type
        let e820_type = uefi_type_to_e820(desc.memory_type);
        
        // Calculate base address and length
        let base = desc.physical_start;
        let length = desc.number_of_pages * 4096; // 4KB pages
        
        // Skip zero-length regions
        if length == 0 {
            continue;
        }
        
        // Try to merge with previous entry if same type and adjacent
        if e820_count > 0 {
            let prev = &mut E820_BUFFER[e820_count - 1];
            if prev.entry_type == e820_type && prev.base + prev.length == base {
                // Merge with previous entry
                prev.length += length;
                continue;
            }
        }
        
        // Create new E820 entry
        E820_BUFFER[e820_count] = E820Entry {
            base,
            length,
            entry_type: e820_type,
            acpi: 0,
        };
        e820_count += 1;
    }
    
    e820_count
}

/// Store E820 memory map at physical address 0x9000
pub unsafe fn store_e820_map(
    e820_count: usize,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) {
    const E820_MAP_ADDRESS: usize = 0x9000;
    
    // Write entry count at 0x9000 (first 4 bytes)
    let count_ptr = E820_MAP_ADDRESS as *mut u32;
    *count_ptr = e820_count as u32;
    
    // Write E820 entries starting at 0x9004
    let entries_ptr = (E820_MAP_ADDRESS + 4) as *mut E820Entry;
    for i in 0..e820_count {
        *entries_ptr.add(i) = E820_BUFFER[i];
    }
    
    println!(console, "Memory map stored at 0x9000");
    println!(console, "E820 entries: ");
    print_decimal(console, e820_count);
}

/// Copy kernel from UEFI buffer to final address (0x200000)
pub unsafe fn copy_kernel_to_final_address(
    kernel_ptr: *const u8,
    kernel_size: usize,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) {
    const KERNEL_LOAD_ADDRESS: usize = 0x200000;
    const MAX_KERNEL_SIZE: usize = 10 * 1024 * 1024; // 10MB
    
    println!(console, "Copying kernel to 0x200000...");
    
    // Verify source pointer is valid
    if kernel_ptr.is_null() {
        error::display_simple_error_and_halt(
            console,
            "Invalid kernel pointer - Cannot copy kernel to final address",
        );
    }
    
    // Verify kernel size is reasonable, non-zero
    if kernel_size == 0 {
        error::display_simple_error_and_halt(
            console,
            "Kernel size is zero - Cannot copy empty kernel",
        );
    }
    
    if kernel_size > MAX_KERNEL_SIZE {
        error::display_simple_error_and_halt(
            console,
            "Kernel size exceeds maximum (10MB) - Kernel too large",
        );
    }
    
    // Get destination pointer
    let dest_ptr = KERNEL_LOAD_ADDRESS as *mut u8;
    
    // Copy kernel byte by byte
    core::ptr::copy_nonoverlapping(kernel_ptr, dest_ptr, kernel_size);
    
    // Verify copy by checking first few bytes
    let verify_ok = {
        let mut ok = true;
        for i in 0..core::cmp::min(16, kernel_size) {
            if *kernel_ptr.add(i) != *dest_ptr.add(i) {
                ok = false;
                break;
            }
        }
        ok
    };
    
    if !verify_ok {
        error::display_simple_error_and_halt(
            console,
            "Kernel copy verification failed - Memory corruption detected",
        );
    }
    
    println!(console, "Kernel copied successfully (");
    print_decimal(console, kernel_size);
    println!(console, " bytes)");
    
    // Display first 4 bytes for verification
    println!(console, "First bytes at 0x200000: 0x");
    print_hex_byte(console, *dest_ptr);
    print_hex_byte(console, *dest_ptr.add(1));
    print_hex_byte(console, *dest_ptr.add(2));
    print_hex_byte(console, *dest_ptr.add(3));
    println!(console, "");
}

/// Print a hexadecimal byte
unsafe fn print_hex_byte(console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, byte: u8) {
    let hex_chars = b"0123456789ABCDEF";
    let mut buffer = [0u16; 3]; // 2 hex digits + null terminator
    
    buffer[0] = hex_chars[(byte >> 4) as usize] as u16;
    buffer[1] = hex_chars[(byte & 0x0F) as usize] as u16;
    buffer[2] = 0; // Null terminator
    
    ((*console).output_string)(console, buffer.as_ptr());
}

/// Print a decimal number
unsafe fn print_decimal(console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, num: usize) {
    if num == 0 {
        let mut buffer = [b'0' as u16, 0];
        ((*console).output_string)(console, buffer.as_ptr());
        return;
    }
    
    let mut buffer = [0u16; 21]; // Max 20 digits + null terminator
    let mut n = num;
    let mut i = 0;
    
    while n > 0 {
        buffer[i] = (b'0' + (n % 10) as u8) as u16;
        n /= 10;
        i += 1;
    }
    
    // Reverse the digits
    for j in 0..i/2 {
        buffer.swap(j, i - 1 - j);
    }
    
    buffer[i] = 0; // Null terminator
    ((*console).output_string)(console, buffer.as_ptr());
}

/// Page table entry flags
const PAGE_PRESENT: u64 = 1 << 0;      // Page is present in memory
const PAGE_WRITABLE: u64 = 1 << 1;     // Page is writable
const PAGE_SIZE: u64 = 1 << 7;         // Page size bit (for 2MB pages in PD)

/// Set up page tables for long mode
pub unsafe fn setup_page_tables(console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL) {
    const PML4_ADDRESS: usize = 0x70000;
    const PDPT_ADDRESS: usize = 0x71000;
    const PD_ADDRESS: usize = 0x72000;
    
    println!(console, "Setting up page tables...");

    const PAGES_TO_MAP: usize = 128; // 128 * 2MB = 256MB
    
    // Zero out three page tables
    let pml4_ptr = PML4_ADDRESS as *mut u64;
    let pdpt_ptr = PDPT_ADDRESS as *mut u64;
    let pd_ptr = PD_ADDRESS as *mut u64;
    
    // Zero out PML4, 512
    for i in 0..512 {
        *pml4_ptr.add(i) = 0;
    }
    
    // Zero out PDPT, 512
    for i in 0..512 {
        *pdpt_ptr.add(i) = 0;
    }
    
    // Zero out PD, 512
    for i in 0..512 {
        *pd_ptr.add(i) = 0;
    }
    
    // Set up PML4[0] to point to PDPT
    *pml4_ptr = (PDPT_ADDRESS as u64) | PAGE_PRESENT | PAGE_WRITABLE;
    
    // Set up PDPT[0] to point to PD
    *pdpt_ptr = (PD_ADDRESS as u64) | PAGE_PRESENT | PAGE_WRITABLE;
    
    // Set up PD entries to identity map first 64MB using 2MB pages
    for i in 0..PAGES_TO_MAP {
        let physical_address = (i * 2 * 1024 * 1024) as u64;
        *pd_ptr.add(i) = physical_address | PAGE_PRESENT | PAGE_WRITABLE | PAGE_SIZE;
    }
    
    println!(console, "Page tables created:");
    println!(console, "  PML4 at 0x70000");
    println!(console, "  PDPT at 0x71000");
    println!(console, "  PD at 0x72000");
    println!(console, "  Identity mapped 0-256MB (2MB pages)");
}
