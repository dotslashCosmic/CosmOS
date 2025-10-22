#![no_std]
#![feature(abi_x86_interrupt)]

//! CosmOS Kernel Library

pub mod arch;
pub mod mm;
pub mod serial;
pub mod vga;

/// Halt the CPU in a loop
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
