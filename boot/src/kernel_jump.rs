//! Kernel Jump Module

use crate::uefi::{
    EFI_BOOT_SERVICES, EFI_HANDLE, EFI_SUCCESS,
    console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
};
use crate::{println, error};

/// Static buffer for memory map during boot services exit
static mut EXIT_MEMORY_MAP_BUFFER: [u8; 8192] = [0; 8192];

/// Initialize COM1 serial port for bare-metal
pub fn init_serial() {
    unsafe {
        // Disable interrupts
        outb(0x3F9, 0x00);
        // Enable DLAB (set baud rate divisor)
        outb(0x3FB, 0x80);
        // Set divisor to 3 (38400 baud)
        outb(0x3F8, 0x03);
        outb(0x3F9, 0x00);
        // 8 bits, no parity, one stop bit
        outb(0x3FB, 0x03);
        // Enable FIFO
        outb(0x3FA, 0xC7);
        // IRQs enabled, RTS/DSR set
        outb(0x3FC, 0x0B);
    }
}

/// Write a string directly to COM1 serial port
pub fn serial_write_str(s: &str) {
    unsafe {
        for byte in s.bytes() {
            // Wait for transmit buffer to be empty
            while (inb(0x3FD) & 0x20) == 0 {}
            outb(0x3F8, byte);
        }
    }
}

#[inline(always)]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(nomem, nostack, preserves_flags)
    );
    value
}

/// Exit UEFI boot services and immediately set up CPU for kernel
pub unsafe fn exit_boot_services_and_setup_cpu(
    boot_services: *mut EFI_BOOT_SERVICES,
    image_handle: EFI_HANDLE,
    map_key: usize,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    page_table_base: u64,
    stack_top: u64,
) -> ! {
    println!(console, "Exiting UEFI boot services...");
    
    // Exit boot services
    let mut current_map_key = map_key;
    let max_retries = 3;
    
    for attempt in 0..max_retries {
        let status = ((*boot_services).exit_boot_services)(image_handle, current_map_key);
        
        if status == EFI_SUCCESS {
            
            // Initialize serial immediately
            init_serial();
            serial_write_str("\nCosmosBootloaderUEFI\n");
            
            // Load our page tables
            serial_write_str("Loading page tables into CR3...\n");
            core::arch::asm!(
                "mov cr3, {}",
                in(reg) page_table_base,
                options(nostack)
            );
            
            // Set up CPU state
            serial_write_str("Setting up CPU state...\n");
            core::arch::asm!("cli", options(nomem, nostack));
            core::arch::asm!(
                "mov rsp, {}",
                in(reg) stack_top,
                options(nomem)
            );
            core::arch::asm!("cld", options(nomem, nostack));
            serial_write_str("Jumping to kernel...\n");
            
            // Jump to kernel
            jump_to_kernel(0x200000);
        }
        
        // Failed, try to get updated memory map
        if attempt < max_retries - 1 {
            let mut map_size = EXIT_MEMORY_MAP_BUFFER.len();
            let mut new_map_key: usize = 0;
            let mut descriptor_size: usize = 0;
            let mut descriptor_version: u32 = 0;
            
            let map_status = ((*boot_services).get_memory_map)(
                &mut map_size,
                EXIT_MEMORY_MAP_BUFFER.as_mut_ptr(),
                &mut new_map_key,
                &mut descriptor_size,
                &mut descriptor_version,
            );
            
            if map_status == EFI_SUCCESS {
                current_map_key = new_map_key;
                continue;
            }
        }
    }
    
    // Failed to exit boot services
    error::display_error_and_halt(
        console,
        "Failed to exit UEFI boot services",
        0,
    );
}

/// Exit UEFI boot services and prepare for kernel jump
pub unsafe fn exit_boot_services_and_jump(
    boot_services: *mut EFI_BOOT_SERVICES,
    image_handle: EFI_HANDLE,
    map_key: usize,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> usize {
    println!(console, "Exiting UEFI boot services...");
    
    let mut current_map_key = map_key;
    let max_retries = 3;
    let mut last_status = EFI_SUCCESS;
    
    // Try to exit boot services, with retry logic
    for attempt in 0..max_retries {
        let status = ((*boot_services).exit_boot_services)(image_handle, current_map_key);
        last_status = status;
        
        if status == EFI_SUCCESS {
            // Success! Boot services are now terminated
            // Note: Console output may no longer work after this point
            return current_map_key;
        }
        
        // If we failed, the memory map likely changed
        // We need to get a new memory map and try again
        if attempt < max_retries - 1 {
            // Get updated memory map
            let mut map_size = EXIT_MEMORY_MAP_BUFFER.len();
            let mut new_map_key: usize = 0;
            let mut descriptor_size: usize = 0;
            let mut descriptor_version: u32 = 0;
            
            let map_status = ((*boot_services).get_memory_map)(
                &mut map_size,
                EXIT_MEMORY_MAP_BUFFER.as_mut_ptr(),
                &mut new_map_key,
                &mut descriptor_size,
                &mut descriptor_version,
            );
            
            if map_status == EFI_SUCCESS {
                current_map_key = new_map_key;
                // Retry with new map key
                continue;
            } else {
                // Failed to get updated memory map
                error::display_error_and_halt(
                    console,
                    "Failed to get updated memory map during boot services exit retry",
                    map_status,
                );
            }
        }
    }
    
    // All retries exhausted
    error::display_error_and_halt(
        console,
        "Failed to exit UEFI boot services after maximum retries",
        last_status,
    );
}

/// Load page tables into CR3
#[inline(never)]
pub unsafe fn load_page_tables(page_table_base: u64) {
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) page_table_base,
        options(nostack)
    );
}

/// Set up minimal CPU state for kernel execution
#[inline(never)]
pub unsafe fn setup_cpu_state_minimal(stack_top: u64) {
    // Disable interrupts - kernel will set up its own IDT
    core::arch::asm!("cli", options(nomem, nostack));
    
    // Set stack pointer to top of stack
    // Stack grows downward from 0xA0000 to 0x90000 (64KB)
    core::arch::asm!(
        "mov rsp, {}",
        in(reg) stack_top,
        options(nomem)
    );
    
    // Clear direction flag - ensures string operations increment
    core::arch::asm!("cld", options(nomem, nostack));
}

/// Jump to kernel entry point
#[inline(never)]
pub unsafe fn jump_to_kernel(kernel_entry: u64) -> ! {
    // Clear all general-purpose registers except RSP
    core::arch::asm!(
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        options(nomem, nostack)
    );
    
    // Jump to kernel entry point, indirect jmp rax
    core::arch::asm!(
        "mov rax, {entry}",
        "jmp rax",
        entry = in(reg) kernel_entry,
        options(noreturn)
    );
}

