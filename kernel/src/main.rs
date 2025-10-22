#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

//! CosmOS Kernel Entry Point

mod arch;
mod serial;
mod vga;

/// Kernel entry point called by CosmosBootloader
/// Enabled long mode (64-bit)
/// Set up basic paging (identity mapped first 2MB)
/// Loaded the kernel at 0x200000 (2MB)
/// This function is the entry point and must never return.
#[no_mangle]
#[link_section = ".text._start"]
pub extern "C" fn _start() -> ! {
    // Use pure assembly to avoid any Rust issues
    unsafe {
        // Disable interrupts immediately
        core::arch::asm!("cli");
        
        // Set up basic stack
        core::arch::asm!("mov rsp, 0x90000");
        
        // Clear screen and write message using inline assembly
        core::arch::asm!(
            "mov rdi, 0xb8000",
            "mov rcx, 2000",        // 80*25 screen
            "mov ax, 0x0F20",       // White space
            "rep stosw",

            "mov rdi, 0xb8000",
            "mov word ptr [rdi], 0x0A4B",      // 'K' in light green
            "mov word ptr [rdi+2], 0x0A45",    // 'E' in light green  
            "mov word ptr [rdi+4], 0x0A52",    // 'R' in light green
            "mov word ptr [rdi+6], 0x0A4E",    // 'N' in light green
            "mov word ptr [rdi+8], 0x0A45",    // 'E' in light green
            "mov word ptr [rdi+10], 0x0A4C",   // 'L' in light green
            "mov word ptr [rdi+12], 0x0A20",   // ' ' in light green
            "mov word ptr [rdi+14], 0x0A45",   // 'E' in light green
            "mov word ptr [rdi+16], 0x0A4E",   // 'N' in light green
            "mov word ptr [rdi+18], 0x0A54",   // 'T' in light green
            "mov word ptr [rdi+20], 0x0A52",   // 'R' in light green
            "mov word ptr [rdi+22], 0x0A59",   // 'Y' in light green

            "mov rdi, 0xb80A0",     // Second line, 160 bytes offset
            "mov word ptr [rdi], 0x0E41",      // 'A' in yellow
            "mov word ptr [rdi+2], 0x0E54",    // 'T' in yellow
            "mov word ptr [rdi+4], 0x0E20",    // ' ' in yellow
            "mov word ptr [rdi+6], 0x0E30",    // '0' in yellow
            "mov word ptr [rdi+8], 0x0E78",    // 'x' in yellow
            "mov word ptr [rdi+10], 0x0E32",   // '2' in yellow
            "mov word ptr [rdi+12], 0x0E30",   // '0' in yellow
            "mov word ptr [rdi+14], 0x0E30",   // '0' in yellow
            "mov word ptr [rdi+16], 0x0E30",   // '0' in yellow
            "mov word ptr [rdi+18], 0x0E30",   // '0' in yellow
            "mov word ptr [rdi+20], 0x0E30",   // '0' in yellow
            
            options(nostack, preserves_flags)
        );

        core::arch::asm!(
            "mov rdi, 0xb8140",     // Third line, 320 bytes offset
            "mov word ptr [rdi], 0x0C48",      // 'H' in light red
            "mov word ptr [rdi+2], 0x0C41",    // 'A' in light red
            "mov word ptr [rdi+4], 0x0C4C",    // 'L' in light red
            "mov word ptr [rdi+6], 0x0C54",    // 'T' in light red
            "mov word ptr [rdi+8], 0x0C49",    // 'I' in light red
            "mov word ptr [rdi+10], 0x0C4E",   // 'N' in light red
            "mov word ptr [rdi+12], 0x0C47",   // 'G' in light red
            options(nostack)
        );
        
        // Infinite halt loop
        loop {
            core::arch::asm!("cli; hlt", options(nostack, nomem));
        }
    }
}

/// Halt the CPU in a loop
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Panic handler for the kernel
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("[KERNEL PANIC] {}", info);
    hlt_loop();
}
