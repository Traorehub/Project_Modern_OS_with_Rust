//! Jour 8 — Double Fault + IST
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

    serial::println("Jour 8 - Double Fault & IST");
    vga::WRITER.lock().write_fmt(format_args!("Jour 8 - Double Fault & IST\n")).ok();
    serial::println("============================");
    vga::WRITER.lock().write_fmt(format_args!("============================\n\n")).ok();

    gdt::init();
    serial::println("GDT + TSS charges.");
    vga::WRITER.lock().write_fmt(format_args!("GDT + TSS charges.\n")).ok();

    idt::init();
    serial::println("IDT chargee.");
    vga::WRITER.lock().write_fmt(format_args!("IDT chargee.\n")).ok();

    // Warm up TLB pour la stack IST avant le test
    unsafe {
        let warmup = 0x37f000 as *mut u64;
        core::ptr::write_volatile(warmup, 0);
        let warmup2 = 0x380000 as *mut u64;
        core::ptr::write_volatile(warmup2, 0);
    }
    serial::println("\nTest : Double Fault via RSP invalide...");
    vga::WRITER.lock().write_fmt(format_args!("\nTest Double Fault...\n")).ok();

    unsafe { trigger_double_fault(); }

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
