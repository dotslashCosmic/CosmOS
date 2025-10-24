//! Error Handling Module

use crate::uefi::{
    EFI_STATUS, EFI_SUCCESS, EFI_LOAD_ERROR, EFI_INVALID_PARAMETER,
    EFI_UNSUPPORTED, EFI_BAD_BUFFER_SIZE, EFI_BUFFER_TOO_SMALL,
    EFI_NOT_READY, EFI_DEVICE_ERROR, EFI_WRITE_PROTECTED,
    EFI_OUT_OF_RESOURCES, EFI_NOT_FOUND,
    console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
};
use crate::{println, halt};

/// Convert EFI_STATUS code to human-readable string
pub fn status_to_string(status: EFI_STATUS) -> &'static str {
    match status {
        EFI_SUCCESS => "Success",
        EFI_LOAD_ERROR => "Load Error",
        EFI_INVALID_PARAMETER => "Invalid Parameter",
        EFI_UNSUPPORTED => "Unsupported",
        EFI_BAD_BUFFER_SIZE => "Bad Buffer Size",
        EFI_BUFFER_TOO_SMALL => "Buffer Too Small",
        EFI_NOT_READY => "Not Ready",
        EFI_DEVICE_ERROR => "Device Error",
        EFI_WRITE_PROTECTED => "Write Protected",
        EFI_OUT_OF_RESOURCES => "Out of Resources",
        EFI_NOT_FOUND => "Not Found",
        _ => "Unknown Error",
    }
}

/// Display error message and halt
pub unsafe fn display_error_and_halt(
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    operation: &str,
    status: EFI_STATUS,
) -> ! {
    use crate::uefi::console::print;
    
    print(console, "\r\n");
    print(console, "BOOTLOADER ERROR\r\n");
    print(console, "Operation: ");
    print(console, operation);
    print(console, "\r\nStatus Code: 0x");
    print_hex_status(console, status);
    print(console, "\r\nDescription: ");
    print(console, status_to_string(status));
    print(console, "\r\nSystem halted.\r\n");
    halt();
}

/// Display a simple error message and halt
pub unsafe fn display_simple_error_and_halt(
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    message: &str,
) -> ! {
    use crate::uefi::console::print;
    
    print(console, "\r\n");
    print(console, "BOOTLOADER ERROR\r\n");
    print(console, message);
    print(console, "\r\nSystem halted.\r\n");
    halt();
}

/// Pprint EFI_STATUS as hexadecimal
unsafe fn print_hex_status(console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, status: EFI_STATUS) {
    let hex_chars = b"0123456789ABCDEF";
    let mut buffer = [0u16; 17]; // 16 hex digits + null terminator
    
    for i in 0..16 {
        let nibble = (status >> (60 - i * 4)) & 0xF;
        buffer[i] = hex_chars[nibble] as u16;
    }
    buffer[16] = 0; // Null terminator
    
    ((*console).output_string)(console, buffer.as_ptr());
}
