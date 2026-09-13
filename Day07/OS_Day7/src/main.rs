//! Jour 7 — IDT et exceptions CPU
#![no_std]
#![feature(abi_x86_interrupt)]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;
mod idt;

use core::panic::PanicInfo;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    serial::println("Jour 7 - IDT & Exceptions CPU");
    serial::println("==============================");

    // Initialiser l'IDT
    idt::init();
    serial::println("IDT chargee.");

    // Test 1 : Division par zero
    serial::println("\nTest 1 : declenchement division par zero...");
    unsafe {
        core::arch::asm!(
            "mov rax, 0",
            "mov rbx, 0",
            "div rbx",
        );
    }

    // Ne devrait pas arriver
    serial::println("Erreur : exception non declenchee !");
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
    unsafe { halt(); }
}
