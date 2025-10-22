//! Interrupt handling initialization

/// Initialize interrupt handling
pub fn init() {
    x86_64::instructions::interrupts::enable();
}

/// Disable interrupts
pub fn disable() {
    x86_64::instructions::interrupts::disable();
}

/// Check if interrupts are enabled
pub fn are_enabled() -> bool {
    x86_64::instructions::interrupts::are_enabled()
}
