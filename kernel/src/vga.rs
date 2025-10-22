//! VGA text mode driver

use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;

/// VGA color enumeration
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// Color code combining foreground and background colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    /// Create a new color code from foreground and background colors
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A screen character with ASCII character and color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// VGA text buffer dimensions
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// VGA buffer wrapper
struct VgaBuffer(*mut u16);

unsafe impl Send for VgaBuffer {}
unsafe impl Sync for VgaBuffer {}

/// Simple VGA text writer
pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    buffer: VgaBuffer,
}

impl Writer {
    /// Write a byte
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let offset = self.row_position * BUFFER_WIDTH + self.column_position;
                let color_byte = self.color_code.0 as u16;
                let char_with_color = (color_byte << 8) | byte as u16;
                
                unsafe {
                    *self.buffer.0.add(offset) = char_with_color;
                }
                
                self.column_position += 1;
            }
        }
    }

    /// Write a string to the current position
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // Not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Move to new line
    fn new_line(&mut self) {
        self.column_position = 0;
        if self.row_position < BUFFER_HEIGHT - 1 {
            self.row_position += 1;
        } else {
            self.scroll_up();
        }
    }

    /// Scroll the screen up by one line
    fn scroll_up(&mut self) {
        unsafe {
            // Move all lines up by one
            for row in 1..BUFFER_HEIGHT {
                let src_offset = row * BUFFER_WIDTH;
                let dst_offset = (row - 1) * BUFFER_WIDTH;
                
                for col in 0..BUFFER_WIDTH {
                    let src_val = *self.buffer.0.add(src_offset + col);
                    *self.buffer.0.add(dst_offset + col) = src_val;
                }
            }
            
            // Clear the last line
            let blank_char = (self.color_code.0 as u16) << 8 | b' ' as u16;
            let last_line_offset = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH;
            for col in 0..BUFFER_WIDTH {
                *self.buffer.0.add(last_line_offset + col) = blank_char;
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        row_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: VgaBuffer(0xb8000 as *mut u16),
    });
}

/// Internal print function
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

/// Print macro for formatted output
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

/// Print macro for formatted output with newline
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
