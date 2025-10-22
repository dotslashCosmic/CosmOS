//! x86_64 architecture-specific implementations

pub mod gdt;
pub mod idt;
pub mod interrupts;

/// Initialize architecture-specific components
pub fn init() {
    gdt::init();
    idt::init();
    interrupts::init();
}
