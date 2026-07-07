//! src/vga2.rs – Writer, colour handling, macros (Day 6)


use core::fmt::{self, Write};

pub const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

pub const fn make_color(fg: Color, bg: Color) -> u8 {
    (bg as u8) << 4 | (fg as u8)
}

pub struct Writer {
    col: usize,
    row: usize,
    color: u8,
}

impl Writer {
    pub const fn new() -> Self {
        Self { col: 0, row: 0, color: make_color(Color::White, Color::Black) }
    }

    #[inline]
    fn buffer_ptr(&self) -> *mut u8 { VGA_BUFFER }

    /// Write a single cell (character + colour) safely.
    unsafe fn write_cell(&mut self, ascii: u8, color: u8) {
        if self.row >= VGA_HEIGHT || self.col >= VGA_WIDTH { return; }
        let offset = (self.row * VGA_WIDTH + self.col) * 2;
        core::ptr::write_volatile(self.buffer_ptr().add(offset), ascii);
        core::ptr::write_volatile(self.buffer_ptr().add(offset + 1), color);
    }

    /// Write a byte, handling newline & scrolling.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => { self.new_line(); }
            b'\r' => { self.col = 0; }
            _ => {
                unsafe { self.write_cell(byte, self.color); }
                self.col += 1;
                if self.col >= VGA_WIDTH { self.new_line(); }
            }
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row + 1 >= VGA_HEIGHT {
            self.scroll_up();
        } else {
            self.row += 1;
        }
    }

    fn scroll_up(&mut self) {
        // copy each line upward
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let src = (row * VGA_WIDTH + col) * 2;
                let dst = ((row - 1) * VGA_WIDTH + col) * 2;
                unsafe {
                    let ch = core::ptr::read_volatile(self.buffer_ptr().add(src));
                    let colr = core::ptr::read_volatile(self.buffer_ptr().add(src + 1));
                    core::ptr::write_volatile(self.buffer_ptr().add(dst), ch);
                    core::ptr::write_volatile(self.buffer_ptr().add(dst + 1), colr);
                }
            }
        }
        // clear last line
        let start = (VGA_HEIGHT - 1) * VGA_WIDTH * 2;
        for i in 0..VGA_WIDTH {
            unsafe {
                core::ptr::write_volatile(self.buffer_ptr().add(start + i * 2), b' ');
                core::ptr::write_volatile(self.buffer_ptr().add(start + i * 2 + 1), self.color);
            }
        }
    }

    pub fn clear(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let offset = (row * VGA_WIDTH + col) * 2;
                unsafe {
                    core::ptr::write_volatile(self.buffer_ptr().add(offset), b' ');
                    core::ptr::write_volatile(self.buffer_ptr().add(offset + 1), self.color);
                }
            }
        }
        self.col = 0;
        self.row = 0;
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color = make_color(fg, bg);
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_byte(b);
        }
        Ok(())
    }
}

// Global mutex‑protected writer (used by the whole kernel)
use crate::vga1::Mutex;
pub static WRITER: Mutex<Writer> = Mutex::new(Writer::new());

// Public macros – re‑exported from the crate root
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        $crate::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!(concat!($($arg)*, "\n")));
}
