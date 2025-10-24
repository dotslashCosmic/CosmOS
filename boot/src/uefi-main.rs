//! CosmOSBootloaderUEFI - Custom Rust bootloader

#![no_std]
#![no_main]

use core::ffi::c_void;

#[macro_use]
mod uefi;
mod error;
mod kernel_loader;
mod memory_setup;
mod kernel_jump;

use uefi::{EFI_SYSTEM_TABLE, EFI_STATUS, EFI_SUCCESS};

/// UEFI entry point
#[no_mangle]
pub extern "efiapi" fn efi_main(
    image_handle: *mut c_void,
    system_table: *mut EFI_SYSTEM_TABLE,
) -> usize {
    // Verify system table is valid
    if system_table.is_null() {
        return 1; // EFI_LOAD_ERROR
    }

    unsafe {
        // Extract system table and boot services pointers
        let console = (*system_table).con_out;
        let boot_services = (*system_table).boot_services;
        
        // Verify console is available
        if console.is_null() {
            error::display_simple_error_and_halt(
                console,
                "Console output not available - System table console pointer is null",
            );
        }
        
        // Verify boot services are available
        if boot_services.is_null() {
            error::display_simple_error_and_halt(
                console,
                "Boot services not available - System table boot services pointer is null",
            );
        }
        
        // Display initialization message
        println!(console, "CosmosBootloaderUEFI v0.0.3");
        println!(console, "Initializing...");
        
        // Load kernel from ESP
        let kernel_buffer = kernel_loader::load_kernel_from_esp_root(boot_services, console);
        
        println!(console, "Kernel loaded at address: ");
        print_hex(console, kernel_buffer.data_ptr as usize);
        
        // Get UEFI memory map
        println!(console, "Retrieving memory map...");
        let memory_info = memory_setup::get_uefi_memory_map(boot_services, console);
        
        // Convert UEFI memory map to E820 format
        println!(console, "Converting memory map to E820 format...");
        let e820_count = memory_setup::convert_uefi_to_e820(
            memory_info.descriptor_size,
            memory_info.descriptor_count,
        );
        
        if e820_count == 0 {
            error::display_simple_error_and_halt(
                console,
                "Failed to convert memory map - No E820 entries created",
            );
        }
        
        // Store E820 map at 0x9000
        memory_setup::store_e820_map(e820_count, console);
        
        // Copy kernel to final address
        memory_setup::copy_kernel_to_final_address(
            kernel_buffer.data_ptr,
            kernel_buffer.size,
            console,
        );
        
        // Setup page tables for long mode
        memory_setup::setup_page_tables(console);
        
        // Exit boot services, switch page tables atomically at the same time
        println!(console, "Exiting boot services and loading page tables...");
        kernel_jump::exit_boot_services_and_setup_cpu(
            boot_services,
            image_handle,
            memory_info.map_key,
            console,
            0x70000,  // page table base
            0xA0000,  // stack top
        );
    }

    // Should never run
    #[allow(unreachable_code)]
    EFI_SUCCESS
}

/// Halt the system in case of unrecoverable error
pub fn halt() -> ! {
    loop {
        unsafe {
            // Halt
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Print a hexadecimal number
unsafe fn print_hex(console: *mut uefi::console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, num: usize) {
    let hex_chars = b"0123456789ABCDEF";
    let mut buffer = [0u16; 17]; // 16 hex digits + null terminator
    
    for i in 0..16 {
        let nibble = (num >> (60 - i * 4)) & 0xF;
        buffer[i] = hex_chars[nibble] as u16;
    }
    buffer[16] = 0; // Null terminator
    
    ((*console).output_string)(console, buffer.as_ptr());
}

/// Panic handler for no_std environment
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // TODO: Display panic message via UEFI serial/console
    halt();
}
