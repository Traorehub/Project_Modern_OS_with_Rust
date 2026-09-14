// Compatibility layer exposing free functions used by tests

pub use crate::vga2::VGA_BUFFER;
pub use crate::vga2::VGA_WIDTH;
pub use crate::vga2::VGA_HEIGHT;
pub use crate::vga2::Color;
pub use crate::vga2::make_color;
pub use crate::vga2::WRITER;

/// Écrit directement un caractère dans le buffer VGA (API compat)
pub unsafe fn write_char(row: usize, col: usize, ascii: u8, color: u8) {
    if row >= VGA_HEIGHT || col >= VGA_WIDTH {
        return;
    }
    let offset = (row * VGA_WIDTH + col) * 2;
    core::ptr::write_volatile(VGA_BUFFER.add(offset), ascii);
    core::ptr::write_volatile(VGA_BUFFER.add(offset + 1), color);
}

/// Écrit une string en commençant à (row, col) — s'arrête en dehors des limites
pub unsafe fn write_str(row: usize, col: usize, s: &str, color: u8) {
    let mut r = row;
    let mut c = col;
    for b in s.bytes() {
        if b == b'\n' {
            c = 0;
            r += 1;
            if r >= VGA_HEIGHT { break; }
            continue;
        }
        if c >= VGA_WIDTH {
            c = 0;
            r += 1;
            if r >= VGA_HEIGHT { break; }
        }
        write_char(r, c, b, color);
        c += 1;
    }
}
