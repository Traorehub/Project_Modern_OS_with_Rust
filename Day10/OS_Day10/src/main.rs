//! Jour 10 — Pagination II : bit NX et parcours des tables
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
mod paging;

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

    serial::println("Jour 10 - Pagination II : bit NX");
    vga::WRITER.lock().write_fmt(format_args!("Jour 10 - Pagination II : bit NX\n")).ok();
    serial::println("=================================");
    vga::WRITER.lock().write_fmt(format_args!("=================================\n\n")).ok();

    gdt::init();
    idt::init();
    serial::println("GDT + IDT initialises.");
    vga::WRITER.lock().write_fmt(format_args!("GDT + IDT initialises.\n")).ok();

    // Afficher le mapping avant NX
    serial::println("\n--- Avant activation NX ---");
    paging::print_page_tables();

    // Activer NX sur les pages de données (hors .text)
    serial::println("Activation du bit NX sur les pages de donnees...");
    unsafe { paging::enable_nx_on_data_pages(); }
    serial::println("Bit NX active.");
    vga::WRITER.lock().write_fmt(format_args!("Bit NX active.\n")).ok();

    // Afficher le mapping après NX
    serial::println("\n--- Apres activation NX ---");
    paging::print_nx_status();

    serial::println("\nPagination II OK.");
    vga::WRITER.lock().write_fmt(format_args!("\nPagination II OK.\n")).ok();

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n=== KERNEL PANIC ===");
    if let Some(msg) = info.message().as_str() {
        serial::print("Raison  : ");
        serial::println(msg);
    }
    if let Some(loc) = info.location() {
        serial::print("Fichier : ");
        serial::println(loc.file());
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
