//! Jour 6 - Kernel principal avec infrastructure de test
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;

use core::panic::PanicInfo;
use vga::Color;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    use core::fmt::Write as _;
    vga::WRITER.lock().write_fmt(format_args!("Jour 6 - Kernel Tests\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("=====================\n\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("Lance : cargo test\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("pour executer la suite de tests.\n\n")).ok();

    vga::WRITER.lock().set_color(Color::Green, Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Infrastructure de test prete.\n")).ok();
    vga::WRITER.lock().set_color(Color::White, Color::Black);

    serial::println("Jour 6 kernel boot OK");

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::WRITER.lock().set_color(Color::White, Color::Red);
    use core::fmt::Write as _;
    vga::WRITER.lock().write_fmt(format_args!("\n=== KERNEL PANIC ===\n")).ok();

    if let Some(msg) = info.message().as_str() {
        vga::WRITER.lock().write_fmt(format_args!("Raison  : {}\n", msg)).ok();
    }
    if let Some(loc) = info.location() {
        vga::WRITER.lock().write_fmt(format_args!("Fichier : {}\n", loc.file())).ok();
        vga::WRITER.lock().write_fmt(format_args!("Ligne   : {}\n", loc.line())).ok();
    }

    vga::WRITER.lock().write_fmt(format_args!("Systeme arrete.\n")).ok();
    unsafe { halt(); }
}
