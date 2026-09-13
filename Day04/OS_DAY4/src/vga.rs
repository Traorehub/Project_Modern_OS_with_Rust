//! VGA text‑mode driver – Day 4
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]   // silence Rust‑2024 warnings inside unsafe fn

/// Base address of the VGA text buffer (80×25, 2 bytes per cell)
pub const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
pub const VGA_WIDTH:  usize = 80;
pub const VGA_HEIGHT: usize = 25;

/// 4‑bit VGA colour palette
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Color {
    Black   = 0,
    Blue    = 1,
    Green   = 2,
    Cyan    = 3,
    Red     = 4,
    Magenta = 5,
    Brown   = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

/// Pack foreground and background colours into a single attribute byte
pub const fn make_color(fg: Color, bg: Color) -> u8 {
    (bg as u8) << 4 | (fg as u8)
}

/// Write one ASCII character at (row, col) with the given colour attribute.
/// Silently returns if coordinates are out of range.
pub unsafe fn write_char(row: usize, col: usize, ascii: u8, color: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let offset = (row * VGA_WIDTH + col) * 2;
    let ptr = VGA_BUFFER.add(offset);
    core::ptr::write_volatile(ptr,        ascii);
    core::ptr::write_volatile(ptr.add(1), color);
}

/// Fill every cell of the screen with a blank (white on black)
pub unsafe fn clear_screen() {
    let blank  = b' ';
    let colour = make_color(Color::White, Color::Black);
    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            write_char(row, col, blank, colour);
        }
    }
}

/// Write a UTF‑8 string starting at (row, col), clamping at screen width
pub unsafe fn write_str(row: usize, col: usize, s: &str, colour: u8) {
    for (i, byte) in s.bytes().enumerate() {
        if col + i >= VGA_WIDTH {
            break;
        }
        write_char(row, col + i, byte, colour);
    }
}
