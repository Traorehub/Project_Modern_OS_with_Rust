//! Day 4 – VGA Buffer kernel (bare‑metal i686)
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]   // silence Rust‑2024 warnings inside unsafe fn

mod vga;

use core::panic::PanicInfo;
use vga::{Color, make_color};

/// COM1 port address for serial debug output
const COM1: u16 = 0x3F8;

unsafe fn write_serial(byte: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") COM1,
        in("al") byte,
    );
}

unsafe fn print_serial(s: &str) {
    for b in s.bytes() {
        write_serial(b);
    }
}

unsafe fn halt() -> ! {
    loop {
        core::arch::asm!("hlt");
    }
}

/// Kernel entry point – placed at 0x8000 by linker.ld
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    unsafe {
        // 1. Clear screen (white on black)
        vga::clear_screen();

        // 2. Write a single character 'A' at (0,0) and mirror to serial
        let yellow = make_color(Color::Yellow, Color::Black);
        vga::write_char(0, 0, b'A', yellow);
        print_serial("A\n");

        // 3. Write greeting at row 1
        let cyan = make_color(Color::Cyan, Color::Black);
        let msg = "Hello from VGA Buffer!";
        vga::write_str(1, 0, msg, cyan);
        print_serial(msg);
        print_serial("\n");

        // 4. Out‑of‑bounds write (should be silently ignored by driver)
        let red = make_color(Color::Red, Color::Black);
        vga::write_char(99, 99, b'X', red);
        vga::write_str(2, 0, "Out-of-bounds write blocked!", red);
        print_serial("Out-of-bounds write blocked!\n");

        // 5. Done
        print_serial("VGA Buffer test done.\n");

        halt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        let panic_color = make_color(Color::White, Color::Red);
        vga::write_str(24, 0, "KERNEL PANIC", panic_color);

        print_serial("\n=== KERNEL PANIC ===\n");
        if let Some(msg) = info.message().as_str() {
            print_serial("Reason: ");
            print_serial(msg);
            print_serial("\n");
        }
        if let Some(loc) = info.location() {
            print_serial("File: ");
            print_serial(loc.file());
            print_serial("\n");
        }
        print_serial("System halted.\n");
        halt();
    }
}
