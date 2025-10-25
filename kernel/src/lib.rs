#![no_std]
#![feature(abi_x86_interrupt)]

//! CosmOS Kernel Library

extern crate alloc;

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

/// Allocate memory on the heap, execute a closure, then deallocate
#[inline]
pub fn with_heap_alloc<T, F, R>(initial_value: T, f: F) -> R
where
    F: FnOnce(&mut T) -> R
{
    let mut boxed = alloc::boxed::Box::new(initial_value);
    f(&mut *boxed)
}
