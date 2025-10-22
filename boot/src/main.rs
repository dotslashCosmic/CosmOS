//! CosmosBootloader - Custom Bootloader for CosmOS

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Bootloader entry point
#[no_mangle]
#[link_section = ".boot"]
pub extern "C" fn _start() -> ! {
    loop {}
}

/// Panic handler for bootloader
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
