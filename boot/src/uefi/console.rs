//! UEFI Console Output Protocol

use super::EFI_STATUS;
use core::ffi::c_void;

/// UEFI Simple Text Output Protocol
#[repr(C)]
pub struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    pub reset: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        extended_verification: bool,
    ) -> EFI_STATUS,
    
    pub output_string: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        string: *const u16,
    ) -> EFI_STATUS,
    
    pub test_string: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        string: *const u16,
    ) -> EFI_STATUS,
    
    pub query_mode: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> EFI_STATUS,
    
    pub set_mode: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        mode_number: usize,
    ) -> EFI_STATUS,
    
    pub set_attribute: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        attribute: usize,
    ) -> EFI_STATUS,
    
    pub clear_screen: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
    ) -> EFI_STATUS,
    
    pub set_cursor_position: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        column: usize,
        row: usize,
    ) -> EFI_STATUS,
    
    pub enable_cursor: extern "efiapi" fn(
        this: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
        visible: bool,
    ) -> EFI_STATUS,
    
    pub mode: *mut c_void,
}

/// Convert UTF-8 string to UTF-16 for UEFI
pub fn utf8_to_utf16(input: &str, output: &mut [u16]) -> usize {
    let mut i = 0;
    for ch in input.chars() {
        if i >= output.len() - 1 {
            break;
        }
        output[i] = ch as u16;
        i += 1;
    }
    output[i] = 0; // Null terminator
    i + 1
}

/// Print a UTF-8 string to the UEFI console
pub unsafe fn print(protocol: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, s: &str) {
    if protocol.is_null() {
        return;
    }

    // Buffer for UTF-16 conversion, 256 characters
    let mut buffer: [u16; 256] = [0; 256];
    
    // Convert UTF-8 to UTF-16
    utf8_to_utf16(s, &mut buffer);
    
    // Call UEFI output_string function
    let output_fn = (*protocol).output_string;
    output_fn(protocol, buffer.as_ptr());
}

/// Macro for printing to UEFI console with newline
#[macro_export]
macro_rules! println {
    ($console:expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        
        // Create a temporary string buffer
        struct StringBuffer {
            buffer: [u8; 512],
            len: usize,
        }
        
        impl StringBuffer {
            fn new() -> Self {
                Self {
                    buffer: [0; 512],
                    len: 0,
                }
            }
            
            fn as_str(&self) -> &str {
                unsafe {
                    core::str::from_utf8_unchecked(&self.buffer[..self.len])
                }
            }
        }
        
        impl core::fmt::Write for StringBuffer {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let remaining = self.buffer.len() - self.len;
                let to_copy = core::cmp::min(bytes.len(), remaining);
                
                self.buffer[self.len..self.len + to_copy].copy_from_slice(&bytes[..to_copy]);
                self.len += to_copy;
                
                Ok(())
            }
        }
        
        let mut buf = StringBuffer::new();
        let _ = core::write!(&mut buf, $($arg)*);
        
        unsafe {
            $crate::uefi::console::print($console, buf.as_str());
            $crate::uefi::console::print($console, "\r\n");
        }
    }};
}
