//! Jour 9 — Pagination I : lecture des tables de pages
#![no_std]
#![feature(abi_x86_interrupt)]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;
mod gdt;
mod idt;
mod paging;   // nouveau module

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

unsafe extern "C" {
    fn trigger_double_fault();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 9 - Pagination I");
    vga::WRITER.lock().write_fmt(format_args!("Jour 9 - Pagination I\n")).ok();
    serial::println("=====================");
    vga::WRITER.lock().write_fmt(format_args!("=====================\n\n")).ok();

    gdt::init();
    idt::init();
    serial::println("GDT + IDT initialises.");
    vga::WRITER.lock().write_fmt(format_args!("GDT + IDT initialises.\n")).ok();

    // Lire et afficher les tables de pages actuelles
    paging::print_page_tables();

    serial::println("\nPagination I OK.");
    vga::WRITER.lock().write_fmt(format_args!("\nPagination I OK.\n")).ok();

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n=== KERNEL PANIC ===");
    if let Some(msg) = info.message().as_str() {
        serial::print("Raison  : ");
        serial::println(msg);
    }
    serial::println("Systeme arrete.");
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Red);
    vga::WRITER.lock().write_fmt(format_args!("\n=== KERNEL PANIC ===\n")).ok();
    if let Some(msg) = info.message().as_str() {
        vga::WRITER.lock().write_fmt(format_args!("Raison : {}\n", msg)).ok();
    }
    vga::WRITER.lock().write_fmt(format_args!("Systeme arrete.\n")).ok();
    unsafe { halt(); }
}
