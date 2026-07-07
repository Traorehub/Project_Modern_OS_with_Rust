//! Jour 14 — Async/Await : Executeur de taches pour le clavier
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
mod frame_allocator;
mod heap;
mod heap_allocator;
mod pic;
mod keyboard;
mod executor;

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 14 - Async/Await : Executeur clavier");
    vga::WRITER.lock().write_fmt(format_args!("Jour 14 - Executeur clavier\n")).ok();
    serial::println("==========================================");
    vga::WRITER.lock().write_fmt(format_args!("==========================================\n\n")).ok();

    gdt::init();
    idt::init();

    serial::println("Initialisation PIC...");
    unsafe { pic::init(); }

    // Activer les interruptions CPU (sti)
    serial::println("Activation des interruptions (sti)...");
    unsafe { core::arch::asm!("sti"); }

    serial::println("Pret. Tapez au clavier dans la fenetre QEMU.\n");
    vga::WRITER.lock().write_fmt(format_args!("Pret. Tapez au clavier !\n")).ok();

    // Lancer l'exécuteur — boucle infinie qui poll le clavier
    let mut executor = executor::Executor::new();
    executor.run();
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
