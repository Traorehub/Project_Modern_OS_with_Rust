//! Tests d'intégration kernel - Jour 6
//! Ces tests tournent directement dans QEMU
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]

mod serial {
    include!("../src/serial.rs");
}
mod test_runner {
    include!("../src/test_runner.rs");
}
mod vga {
    include!("../src/vga.rs");
}

use core::panic::PanicInfo;
use test_runner::{Testable, run_tests, exit_qemu, QemuExitCode};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    run_tests(&[
        &test_trivial_assertion,
        &test_vga_bounds_check,
        &test_make_color,
        &test_vga_write_char,
        &test_writer_newline,
        &test_bounds_out_of_range,
    ]);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n[ECHEC] PANIC dans le test !");

    if let Some(msg) = info.message().as_str() {
        serial::print("  Raison  : ");
        serial::println(msg);
    }
    if let Some(loc) = info.location() {
        serial::print("  Fichier : ");
        serial::println(loc.file());
    }

    exit_qemu(QemuExitCode::Failed);
}

/// Tests
fn test_trivial_assertion() {
    assert_eq!(1 + 1, 2);
}

fn test_vga_bounds_check() {
    unsafe {
        vga::write_char(VGA_HEIGHT, 0, b'X', 0x0F);
        vga::write_char(0, VGA_WIDTH, b'X', 0x0F);
        vga::write_char(99, 99, b'X', 0x0F);
        vga::write_char(usize::MAX, 0, b'X', 0x0F);
    }
}

fn test_make_color() {
    use vga::Color;
    assert_eq!(vga::make_color(Color::White, Color::Black), 0x0F);
    assert_eq!(vga::make_color(Color::Black, Color::White), 0xF0);
    assert_eq!(vga::make_color(Color::Cyan, Color::Red), 0x43);
}

fn test_vga_write_char() {
    unsafe {
        vga::write_char(0, 0, b'Z', 0x0F);
        let ptr = vga::VGA_BUFFER as *const u8;
        let ch    = core::ptr::read_volatile(ptr);
        let color = core::ptr::read_volatile(ptr.add(1));
        assert_eq!(ch, b'Z');
        assert_eq!(color, 0x0F);
    }
}

fn test_writer_newline() {
    use vga::WRITER;
    let mut w = WRITER.lock();
    w.clear();
    w.write_byte(b'A');
    w.write_byte(b'\n');
    w.write_byte(b'B');
}

fn test_bounds_out_of_range() {
    unsafe {
        let long_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        vga::write_str(0, 0, long_str, 0x0F);
    }
}

const VGA_HEIGHT: usize = 25;
const VGA_WIDTH:  usize = 80;
