#![no_std]
#![no_main]

//! CosmOS Kernel Entry Point

use cosmos::mm::MemoryMap;
#[no_mangle]
#[link_section = ".rodata.signature"]
static KERNEL_SIGNATURE: u32 = 0xC05305; // "CosmOS"
#[no_mangle]
#[link_section = ".text._start"]
pub extern "C" fn _start() -> ! {
    // Write directly to VGA buffer
    unsafe {
        let vga_buffer = 0xb8000 as *mut u16;
        
        // Clear screen with black background
        const BUFFER_WIDTH: usize = 80;
        const BUFFER_HEIGHT: usize = 25;
        const BUFFER_SIZE: usize = BUFFER_WIDTH * BUFFER_HEIGHT;
        
        for i in 0..BUFFER_SIZE {
            *vga_buffer.add(i) = 0x0F20; // White space on black
        }
        
        // Write "CosmOS Kernel v0.0.2" in green
        let message = b"CosmOS Kernel v0.0.2";
        for (i, &byte) in message.iter().enumerate() {
            *vga_buffer.add(i) = 0x0B00 | byte as u16;
        }
        
        let mut current_line = 1;
        
        // Helper function to write a line of text
        let write_line = |text: &[u8], color: u16, line_num: &mut usize| {
            for (i, &byte) in text.iter().enumerate() {
                if i < BUFFER_WIDTH && *line_num < BUFFER_HEIGHT {
                    *vga_buffer.add(*line_num * BUFFER_WIDTH + i) = color | byte as u16;
                }
            }
            *line_num += 1;
        };

        // Test memory management
        match MemoryMap::from_bootloader() {
            Ok(_memory_map) => {
                write_line(b"Memory map parsed successfully!", 0x0A00, &mut current_line);
            }
            Err(_) => {
                let _fallback_map = MemoryMap::create_fallback();
            }
        }
        
        write_line(b"Memory Management Module loaded.", 0x0A00, &mut current_line);
        
        // Test memory regions
        let memory_regions = [
            (0x0, "Bootloader Entry"),
            (0x400, "BIOS Equipment"),
            (0x413, "Base Memory KB"),
            (0x417, "Keyboard Flags"),
            (0x41A, "Keyboard Buffer"),
            (0x46C, "Timer Ticks"),
            (0x475, "Hard Disk Count"),
            (0x500, "BIOS Data"),
            (0x7C00, "Boot Sector"),
            (0x7DFE, "Boot Signature"),
            (0x8000, "Stage2 Start"),
            (0x9000, "Memory Entries"),
            (0x10000, "Kernel Copy"),
            (0x9FC00, "Low Memory End"),
            (vga_buffer as usize, "VGA Buffer"),
        ];
        
        for (addr, desc) in memory_regions.iter() {
            // Read tests
            let ptr = *addr as *const u32;
            let value = *ptr;
            
            // Format output line
            let mut msg = [b' '; 80];
            
            // Copy description (limit to 20 chars)
            for (i, &b) in desc.as_bytes().iter().enumerate() {
                if i < 20 {
                    msg[i] = b;
                }
            }
            
            // Add address at position 22
            msg[22] = b'0';
            msg[23] = b'x';
            
            // Convert address to hex, 6 digits
            let hex_chars = b"0123456789ABCDEF";
            for i in 0..6 {
                let nibble = ((*addr >> (20 - i * 4)) & 0xF) as usize;
                msg[24 + i] = hex_chars[nibble];
            }
            
            // Add value at position 32
            msg[32] = b'=';
            msg[33] = b'0';
            msg[34] = b'x';
            
            // Show first 4 hex digits of value
            for i in 0..4 {
                let nibble = ((value >> (12 - i * 4)) & 0xF) as usize;
                msg[35 + i] = hex_chars[nibble];
            }
            
            write_line(&msg[..50], 0x0F00, &mut current_line);
        }

        
        let kernel_addrs = [
            (_start as *const () as usize, "Kernel Entry"),
            (core::mem::size_of::<()> as *const () as usize, "size_of fn"),
            (core::ptr::null::<()> as *const () as usize, "null ptr fn"),
            (&KERNEL_SIGNATURE as *const u32 as usize, "Kernel Signature"),
        ];
        
        for (addr, desc) in kernel_addrs.iter() {
            // Read the actual value at this address
            let ptr = *addr as *const u32;
            let value = *ptr;
            
            let mut msg = [b' '; 80];
            
            // Copy description, limit to 20 chars
            for (i, &b) in desc.as_bytes().iter().enumerate() {
                if i < 20 {
                    msg[i] = b;
                }
            }
            
            // Add address at position 21
            msg[21] = b'0';
            msg[22] = b'x';
            
            // Convert address to hex, 8 digits
            let hex_chars = b"0123456789ABCDEF";
            for i in 0..8 {
                let nibble = ((*addr >> (28 - i * 4)) & 0xF) as usize;
                msg[23 + i] = hex_chars[nibble];
            }

            // Add value at position 32
            msg[32] = b'=';
            msg[33] = b'0';
            msg[34] = b'x';
            
            // Show first 4 hex digits of value
            for i in 0..4 {
                let nibble = ((value >> (12 - i * 4)) & 0xF) as usize;
                msg[35 + i] = hex_chars[nibble];
            }
            
            write_line(&msg[..40], 0x0E00, &mut current_line);
        }
        
        // Final status
        write_line(b"HALTING SAFELY...", 0x0A00, &mut current_line);
    }
    
    // Infinite halt loop
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nostack, nomem));
        }
    }
}

/// Test basic memory regions (fallback when memory map fails)
fn test_basic_memory_regions<F>(write_line: &F, current_line: &mut usize) 
where 
    F: Fn(&[u8], u16, &mut usize)
{
    let memory_regions = [
        (0x400, "BIOS Equipment"),
        (0x413, "Base Memory KB"),
        (0x417, "Keyboard Flags"),
        (0x41A, "Keyboard Buffer"),
        (0x46C, "Timer Ticks"),
        (0x475, "Hard Disk Count"),
        (0x500, "BIOS Data"),
        (0x7C00, "Boot Sector 1"),
        (0x7DFE, "Boot Signature"),
        (0x8000, "Boot Sector 2"),
        (0x9000, "Memory Entries"),
        (0x10000, "Kernel Copy"),
        (0x9FC00, "Memory End"),
        (0xB8000, "VGA Buffer"),
    ];
    
    for (addr, desc) in memory_regions.iter() {
        test_memory_address(*addr, desc, write_line, current_line);
    }
}

/// Test a specific memory address
fn test_memory_address<F>(addr: usize, desc: &str, write_line: &F, current_line: &mut usize)
where 
    F: Fn(&[u8], u16, &mut usize)
{
    unsafe {
        // Try to read from the address
        let ptr = addr as *const u32;
        let value = *ptr;
        
        // Format output line
        let mut msg = [b' '; 80];
        
        // Copy description, limit to 20 chars
        for (i, &b) in desc.as_bytes().iter().enumerate() {
            if i < 20 {
                msg[i] = b;
            }
        }
        
        // Add address at position 22
        msg[22] = b'0';
        msg[23] = b'x';
        
        // Convert address to hex, 8 digits
        let hex_chars = b"0123456789ABCDEF";
        for i in 0..8 {
            let nibble = ((addr >> (28 - i * 4)) & 0xF) as usize;
            msg[24 + i] = hex_chars[nibble];
        }
        
        // Add value at position 34
        msg[34] = b'=';
        msg[35] = b'0';
        msg[36] = b'x';
        
        // Show first 4 hex digits of value
        for i in 0..4 {
            let nibble = ((value >> (12 - i * 4)) & 0xF) as usize;
            msg[37 + i] = hex_chars[nibble];
        }
        
        write_line(&msg[..50], 0x0F00, current_line);
    }
}

/// Display a number in decimal format
fn display_number<F>(prefix: &[u8], number: u64, write_line: &F, current_line: &mut usize)
where 
    F: Fn(&[u8], u16, &mut usize)
{
    let mut msg = [b' '; 80];
    let mut pos = 0;
    
    // Copy prefix
    for &b in prefix {
        if pos < 60 {
            msg[pos] = b;
            pos += 1;
        }
    }
    
    // Convert number to decimal
    if number == 0 {
        msg[pos] = b'0';
        pos += 1;
    } else {
        let mut temp_number = number;
        let mut digits = [0u8; 20];
        let mut digit_count = 0;
        
        while temp_number > 0 {
            digits[digit_count] = (temp_number % 10) as u8 + b'0';
            temp_number /= 10;
            digit_count += 1;
        }
        
        // Reverse digits
        for i in 0..digit_count {
            if pos < 70 {
                msg[pos] = digits[digit_count - 1 - i];
                pos += 1;
            }
        }
    }
    
    write_line(&msg[..pos], 0x0E00, current_line);
}

/// Panic handler for the kernel
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Write panic message directly to VGA
    unsafe {
        const BUFFER_WIDTH: usize = 80;
        let vga_buffer = 0xb8000 as *mut u16;
        let panic_msg = b"KERNEL PANIC!";
        // Write panic message at line 3 (3 * BUFFER_WIDTH)
        for (i, &byte) in panic_msg.iter().enumerate() {
            *vga_buffer.add(3 * BUFFER_WIDTH + i) = 0x0C00 | byte as u16; // Light red
        }
    }
    
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nostack, nomem));
        }
    }
}

