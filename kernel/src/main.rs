#![no_std]
#![no_main]

//! CosmOS Kernel Entry Point

use cosmos::mm::MemoryMap;

/// Simple serial port writer for COM1 (0x3F8)
struct SerialPort {
    base: u16,
}

impl SerialPort {
    const fn new(base: u16) -> Self {
        SerialPort { base }
    }
    
    fn init(&self) {
        unsafe {
            // Disable interrupts
            Self::outb(self.base + 1, 0x00);
            // Enable DLAB (set baud rate divisor)
            Self::outb(self.base + 3, 0x80);
            // Set divisor to 3 (38400 baud)
            Self::outb(self.base + 0, 0x03);
            Self::outb(self.base + 1, 0x00);
            // 8 bits, no parity, one stop bit
            Self::outb(self.base + 3, 0x03);
            // Enable FIFO, clear them, with 14-byte threshold
            Self::outb(self.base + 2, 0xC7);
            // IRQs enabled, RTS/DSR set
            Self::outb(self.base + 4, 0x0B);
        }
    }
    
    fn write_byte(&self, byte: u8) {
        unsafe {
            // Wait for transmit buffer to be empty
            while (Self::inb(self.base + 5) & 0x20) == 0 {}
            Self::outb(self.base, byte);
        }
    }
    
    fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r'); // Add carriage return for proper line breaks
            }
            self.write_byte(byte);
        }
    }
    
    unsafe fn outb(port: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    
    unsafe fn inb(port: u16) -> u8 {
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
        value
    }
}

static SERIAL: SerialPort = SerialPort::new(0x3F8);

/// Dual output writer - writes to both VGA and Serial
struct DualWriter {
    vga_buffer: *mut u16,
    column: usize,
    row: usize,
}

impl DualWriter {
    const BUFFER_WIDTH: usize = 80;
    const BUFFER_HEIGHT: usize = 25;
    
    const fn new() -> Self {
        DualWriter {
            vga_buffer: 0xb8000 as *mut u16,
            column: 0,
            row: 0,
        }
    }
    
    fn write_byte(&mut self, byte: u8, color: u16) {
        unsafe {
            match byte {
                b'\n' => {
                    // Newline - move to next line
                    self.column = 0;
                    self.row += 1;
                    if self.row >= Self::BUFFER_HEIGHT {
                        self.row = Self::BUFFER_HEIGHT - 1;
                    }
                    // Also write to serial
                    SERIAL.write_byte(b'\n');
                }
                byte => {
                    // Write to VGA
                    if self.column < Self::BUFFER_WIDTH && self.row < Self::BUFFER_HEIGHT {
                        let offset = self.row * Self::BUFFER_WIDTH + self.column;
                        *self.vga_buffer.add(offset) = color | byte as u16;
                    }
                    self.column += 1;
                    if self.column >= Self::BUFFER_WIDTH {
                        self.column = 0;
                        self.row += 1;
                        if self.row >= Self::BUFFER_HEIGHT {
                            self.row = Self::BUFFER_HEIGHT - 1;
                        }
                    }
                    // Also write to serial
                    SERIAL.write_byte(byte);
                }
            }
        }
    }
    
    fn write_line(&mut self, text: &[u8], color: u16) {
        for &byte in text {
            if byte != 0 {
                self.write_byte(byte, color);
            }
        }
        self.write_byte(b'\n', color);
    }
    
    fn clear_screen(&mut self) {
        unsafe {
            for i in 0..(Self::BUFFER_WIDTH * Self::BUFFER_HEIGHT) {
                *self.vga_buffer.add(i) = 0x0F20; // White space on black
            }
        }
        self.column = 0;
        self.row = 0;
    }
}

static mut WRITER: DualWriter = DualWriter::new();

#[no_mangle]
#[link_section = ".rodata.signature"]
static KERNEL_SIGNATURE: u32 = 0xC05305; // "CosmOS"
#[no_mangle]
#[link_section = ".text._start"]
pub extern "C" fn _start() -> ! {
    // Initialize serial port FIRST - before anything else
    SERIAL.init();
    
    unsafe {
        // Clear screen (VGA + Serial header)
        WRITER.clear_screen();
        
        // Write title in green (0x0B00)
        let message = b"CosmOS Kernel v0.0.3";
        for &byte in message {
            WRITER.write_byte(byte, 0x0B00);
        }
        WRITER.write_byte(b'\n', 0x0F00);
        
        // Test memory management
        match MemoryMap::from_bootloader() {
            Ok(_memory_map) => {
                WRITER.write_line(b"Bootloader memory map parsed successfully!", 0x0A00);
            }
            Err(_) => {
                let _fallback_map = MemoryMap::create_fallback();
                WRITER.write_line(b"Memory map parsed successfully!", 0x0A00);
            }
        }
        
        // Detect boot mode by checking BIOS data area
        let bios_equipment_ptr = 0x400 as *const u16;
        let bios_equipment = *bios_equipment_ptr;
        
        let boot_mode = if bios_equipment == 0 {
            b"Boot Mode: UEFI"
        } else {
            b"Boot Mode: BIOS"
        };
        WRITER.write_line(boot_mode, 0x0E00); // Yellow

        // BIOS uses VGA, UEFI uses Serial
        if bios_equipment == 0 {
            WRITER.write_line(b"Output Mode: Serial", 0x0E00);
        } else {
            WRITER.write_line(b"Output Mode: VGA", 0x0E00);
        }
        
        // Show E820 entry count
        let e820_count_ptr = 0x9000 as *const u32;
        let e820_count = *e820_count_ptr;
        
        let mut msg = [b' '; 80];
        let prefix = b"E820 Entries: ";
        for (i, &b) in prefix.iter().enumerate() {
            msg[i] = b;
        }
        let mut pos = prefix.len();
        
        // Convert count to decimal
        if e820_count == 0 {
            msg[pos] = b'0';
            pos += 1;
        } else {
            let mut temp = e820_count;
            let mut digits = [0u8; 10];
            let mut digit_count = 0;
            while temp > 0 {
                digits[digit_count] = (temp % 10) as u8 + b'0';
                temp /= 10;
                digit_count += 1;
            }
            for i in 0..digit_count {
                msg[pos] = digits[digit_count - 1 - i];
                pos += 1;
            }
        }
        WRITER.write_line(&msg[..pos], 0x0E00);
        
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
            
            WRITER.write_line(&msg[..40], 0x0E00);
        }
        
        // Final status
        WRITER.write_line(b"Kernel initialization complete.", 0x0A00);
        WRITER.write_line(b"HALTING SAFELY...", 0x0A00);
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
        (0x9000, "Memory Entries"),
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
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Write panic message to serial (works in both BIOS and UEFI)
    SERIAL.write_str("\n!!! KERNEL PANIC !!!\n");
    if let Some(location) = info.location() {
        SERIAL.write_str("Location: ");
        SERIAL.write_str(location.file());
        SERIAL.write_str(":");
        // Simple number to string conversion
        let line = location.line();
        let mut buf = [0u8; 10];
        let mut n = line;
        let mut i = 0;
        if n == 0 {
            buf[0] = b'0';
            i = 1;
        } else {
            while n > 0 {
                buf[i] = (n % 10) as u8 + b'0';
                n /= 10;
                i += 1;
            }
        }
        // Reverse the digits
        for j in 0..i/2 {
            buf.swap(j, i - 1 - j);
        }
        SERIAL.write_str(core::str::from_utf8(&buf[..i]).unwrap_or("?"));
        SERIAL.write_str("\n");
    }
    
    // Also write to VGA if available (BIOS mode)
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

