//! Kernel Loading Module

use crate::uefi::{
    EFI_BOOT_SERVICES, EFI_SUCCESS,
    file::{
        EFI_SIMPLE_FILE_SYSTEM_PROTOCOL, EFI_FILE_PROTOCOL, EFI_FILE_INFO,
        SIMPLE_FILE_SYSTEM_PROTOCOL_GUID, EFI_FILE_MODE_READ, EFI_FILE_INFO_GUID,
    },
    console::EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
};
use crate::{println, error};
use core::ffi::c_void;

/// Kernel buffer information
pub struct KernelBuffer {
    pub data_ptr: *const u8,
    pub size: usize,
}

/// Locate the File System Protocol
pub unsafe fn locate_file_system(
    boot_services: *mut EFI_BOOT_SERVICES,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> *mut EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    let mut fs_protocol: *mut c_void = core::ptr::null_mut();
    
    let status = ((*boot_services).locate_protocol)(
        &SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
        core::ptr::null_mut(),
        &mut fs_protocol as *mut *mut c_void,
    );
    
    if status != EFI_SUCCESS {
        error::display_error_and_halt(
            console,
            "File system not found - Failed to locate Simple File System Protocol",
            status,
        );
    }
    
    fs_protocol as *mut EFI_SIMPLE_FILE_SYSTEM_PROTOCOL
}

/// Open kernel file from ESP root
pub unsafe fn open_kernel_file(
    fs_protocol: *mut EFI_SIMPLE_FILE_SYSTEM_PROTOCOL,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> *mut EFI_FILE_PROTOCOL {
    // Open root volume
    let mut root: *mut EFI_FILE_PROTOCOL = core::ptr::null_mut();
    let status = ((*fs_protocol).open_volume)(fs_protocol, &mut root);
    
    if status != EFI_SUCCESS {
        error::display_error_and_halt(
            console,
            "Failed to open ESP root volume",
            status,
        );
    }
    
    if root.is_null() {
        error::display_simple_error_and_halt(
            console,
            "Failed to open ESP root volume - null pointer returned",
        );
    }
    
    // Convert "kernel.bin" to UTF-16
    let kernel_name: [u16; 11] = [
        'k' as u16, 'e' as u16, 'r' as u16, 'n' as u16, 'e' as u16, 'l' as u16,
        '.' as u16, 'b' as u16, 'i' as u16, 'n' as u16, 0,
    ];
    
    // Open kernel.bin
    let mut file: *mut EFI_FILE_PROTOCOL = core::ptr::null_mut();
    let status = ((*root).open)(
        root,
        &mut file,
        kernel_name.as_ptr(),
        EFI_FILE_MODE_READ,
        0,
    );
    
    // Close root directory
    ((*root).close)(root);
    
    if status != EFI_SUCCESS {
        error::display_error_and_halt(
            console,
            "Kernel not found - Failed to open kernel.bin from ESP root",
            status,
        );
    }
    
    if file.is_null() {
        error::display_simple_error_and_halt(
            console,
            "Kernel not found - kernel.bin file handle is null",
        );
    }
    
    file
}

/// Get the size of a file
pub unsafe fn get_file_size(
    file: *mut EFI_FILE_PROTOCOL,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> usize {
    // Buffer for file info (EFI_FILE_INFO + filename)
    let mut info_buffer: [u8; 512] = [0; 512];
    let mut buffer_size = info_buffer.len();
    
    let status = ((*file).get_info)(
        file,
        &EFI_FILE_INFO_GUID,
        &mut buffer_size,
        info_buffer.as_mut_ptr(),
    );
    
    if status != EFI_SUCCESS {
        error::display_error_and_halt(
            console,
            "Failed to get kernel file information",
            status,
        );
    }
    
    // Cast buffer to EFI_FILE_INFO structure
    let file_info = info_buffer.as_ptr() as *const EFI_FILE_INFO;
    let file_size = (*file_info).file_size as usize;
    
    if file_size == 0 {
        error::display_simple_error_and_halt(
            console,
            "Kernel file is empty - kernel.bin has zero size",
        );
    }
    
    file_size
}

/// Read kernel file into buffer
pub unsafe fn read_kernel_into_buffer(
    file: *mut EFI_FILE_PROTOCOL,
    file_size: usize,
    boot_services: *mut EFI_BOOT_SERVICES,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> *mut u8 {
    // Allocate buffer for kernel
    let mut buffer: *mut u8 = core::ptr::null_mut();
    let status = ((*boot_services).allocate_pool)(
        2, // EfiLoaderData
        file_size,
        &mut buffer,
    );
    
    if status != EFI_SUCCESS {
        error::display_error_and_halt(
            console,
            "Memory allocation failed - Cannot allocate buffer for kernel",
            status,
        );
    }
    
    if buffer.is_null() {
        error::display_simple_error_and_halt(
            console,
            "Memory allocation failed - Kernel buffer pointer is null",
        );
    }
    
    // Read file into buffer
    let mut read_size = file_size;
    let status = ((*file).read)(file, &mut read_size, buffer);
    
    if status != EFI_SUCCESS {
        ((*boot_services).free_pool)(buffer);
        error::display_error_and_halt(
            console,
            "Failed to read kernel file from disk",
            status,
        );
    }
    
    if read_size != file_size {
        ((*boot_services).free_pool)(buffer);
        error::display_simple_error_and_halt(
            console,
            "Incomplete kernel read - File size mismatch",
        );
    }
    
    buffer
}

/// Load kernel from ESP root directory
pub unsafe fn load_kernel_from_esp_root(
    boot_services: *mut EFI_BOOT_SERVICES,
    console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL,
) -> KernelBuffer {
    println!(console, "Loading kernel from ESP...");
    
    // Locate file system protocol
    let fs_protocol = locate_file_system(boot_services, console);
    
    // Open kernel file
    let file = open_kernel_file(fs_protocol, console);
    
    // Get file size
    let file_size = get_file_size(file, console);
    
    println!(console, "Kernel size: ");
    print_number(console, file_size);
    println!(console, " bytes");
    
    // Read file into buffer
    let buffer = read_kernel_into_buffer(file, file_size, boot_services, console);
    
    // Close file
    let close_status = ((*file).close)(file);
    if close_status != EFI_SUCCESS {
        // Non-fatal error, just log it
        println!(console, "Warning: Failed to close kernel file");
    }
    
    println!(console, "Kernel loaded successfully");
    
    KernelBuffer {
        data_ptr: buffer as *const u8,
        size: file_size,
    }
}

/// Print a number to console
unsafe fn print_number(console: *mut EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL, num: usize) {
    let mut buffer = [0u16; 32];
    let mut n = num;
    let mut i = 0;
    
    if n == 0 {
        buffer[0] = '0' as u16;
        buffer[1] = 0;
        ((*console).output_string)(console, buffer.as_ptr());
        return;
    }
    
    // Convert number to string, reverse order
    while n > 0 {
        buffer[i] = ('0' as u16) + ((n % 10) as u16);
        n /= 10;
        i += 1;
    }
    
    // Reverse the string
    let mut j = 0;
    while j < i / 2 {
        let temp = buffer[j];
        buffer[j] = buffer[i - 1 - j];
        buffer[i - 1 - j] = temp;
        j += 1;
    }
    
    buffer[i] = 0; // Null terminator
    ((*console).output_string)(console, buffer.as_ptr());
}
