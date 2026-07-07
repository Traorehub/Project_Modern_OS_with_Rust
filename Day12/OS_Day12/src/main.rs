//! Jour 12 — Allocation II : mapping du heap
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

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 12 - Allocation II : Heap Mapping");
    vga::WRITER.lock().write_fmt(format_args!("Jour 12 - Heap Mapping\n")).ok();
    serial::println("=======================================");
    vga::WRITER.lock().write_fmt(format_args!("=======================================\n\n")).ok();

    gdt::init();
    idt::init();

    // Initialiser l'allocateur de frames
    serial::println("Initialisation allocateur...");
    frame_allocator::init();
    unsafe {
        (*core::ptr::addr_of_mut!(frame_allocator::ALLOCATOR)).print_stats();
    }

    // Mapper le heap
    let ok = heap::map_heap();
    if !ok {
        serial::println("ERREUR : mapping heap echoue !");
        unsafe { halt(); }
    }

    // Afficher stats après mapping
    serial::println("Frames apres mapping heap :");
    unsafe {
        (*core::ptr::addr_of_mut!(frame_allocator::ALLOCATOR)).print_stats();
    }

    // Tester le heap
    heap::test_heap();

    // Différence allocation statique vs dynamique
    serial::println("=== Statique vs Dynamique ===");
    serial::println("  Statique  : taille fixe a la compilation");
    serial::println("  Heap      : taille variable a l'execution");
    serial::println("  Notre heap: 8Mo mappe dynamiquement");
    serial::println("=============================\n");

    // Preuve que le mapping est reel : acces hors heap → Page Fault
    serial::println("Allocation II OK.");
    vga::WRITER.lock().write_fmt(format_args!("Allocation II OK.\n")).ok();

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
