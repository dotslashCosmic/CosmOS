#![no_std]
#![feature(abi_x86_interrupt)]

//! CosmOS Kernel Library

pub mod arch;
pub mod serial;
pub mod vga;

use core::panic::PanicInfo;

/// Kernel panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[PANIC] {}", info);
    
    // Halt the CPU
    loop {
        x86_64::instructions::hlt();
    }
}

/// Halt the CPU in a loop
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
