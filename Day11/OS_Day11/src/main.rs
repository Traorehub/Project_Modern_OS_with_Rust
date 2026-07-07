//! Jour 11 — Allocation I : allocateur de frames physiques
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

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 11 - Allocation I : Frame Allocator");
    vga::WRITER.lock().write_fmt(format_args!("Jour 11 - Frame Allocator\n")).ok();
    serial::println("=========================================");
    vga::WRITER.lock().write_fmt(format_args!("=========================================\n\n")).ok();

    gdt::init();
    idt::init();

    // Initialiser l'allocateur de frames
    serial::println("Initialisation allocateur...");
    frame_allocator::init();

    // Afficher la carte mémoire initiale
    unsafe { (*core::ptr::addr_of_mut!(frame_allocator::ALLOCATOR)).print_map(); }

    // Test 1 : allouer 3 frames
    serial::println("Test 1 : allocation de 3 frames");
    let f1 = frame_allocator::alloc_frame();
    let f2 = frame_allocator::alloc_frame();
    let f3 = frame_allocator::alloc_frame();

    match f1 {
        Some(addr) => { serial::print("  Frame 1 allouee a "); print_hex(addr); serial::println(""); }
        None       => serial::println("  ERREUR : plus de frames !"),
    }
    match f2 {
        Some(addr) => { serial::print("  Frame 2 allouee a "); print_hex(addr); serial::println(""); }
        None       => serial::println("  ERREUR : plus de frames !"),
    }
    match f3 {
        Some(addr) => { serial::print("  Frame 3 allouee a "); print_hex(addr); serial::println(""); }
        None       => serial::println("  ERREUR : plus de frames !"),
    }

    // Test 2 : libérer une frame
    serial::println("\nTest 2 : liberation frame 1");
    if let Some(addr) = f1 {
        let ok = frame_allocator::free_frame(addr);
        if ok {
            serial::print("  Frame a ");
            print_hex(addr);
            serial::println(" liberee");
        }
    }

    // Test 3 : réallouer après libération
    serial::println("\nTest 3 : reallocation apres liberation");
    let f4 = frame_allocator::alloc_frame();
    match f4 {
        Some(addr) => { serial::print("  Frame reallouee a "); print_hex(addr); serial::println(""); }
        None       => serial::println("  ERREUR !"),
    }

    // Test 4 : épuiser toutes les frames
    serial::println("\nTest 4 : epuisement des frames");
    let mut count = 0;
    loop {
        match frame_allocator::alloc_frame() {
            Some(_) => count += 1,
            None    => break,
        }
    }
    serial::print("  ");
    print_num(count);
    serial::println(" frames supplementaires allouees");
    serial::println("  Allocation suivante : None (memoire epuisee)");

    // Afficher la carte finale
    unsafe { (*core::ptr::addr_of_mut!(frame_allocator::ALLOCATOR)).print_map(); }
    unsafe { (*core::ptr::addr_of_mut!(frame_allocator::ALLOCATOR)).print_stats(); }

    serial::println("\nAllocation I OK.");
    vga::WRITER.lock().write_fmt(format_args!("\nAllocation I OK.\n")).ok();

    unsafe { halt(); }
}

fn print_hex(v: u64) {
    serial::print("0x");
    let mut buf = [0u8; 16];
    let mut i = 16usize;
    let mut n = v;
    if n == 0 { serial::print("0"); return; }
    while n > 0 && i > 0 {
        i -= 1;
        let nibble = (n & 0xF) as u8;
        buf[i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        n >>= 4;
    }
    for &b in &buf[i..] {
        serial::print(core::str::from_utf8(&[b]).unwrap_or("?"));
    }
}

fn print_num(n: usize) {
    if n == 0 { serial::print("0"); return; }
    let mut buf = [0u8; 20];
    let mut i = 20usize;
    let mut v = n;
    while v > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &b in &buf[i..] {
        serial::print(core::str::from_utf8(&[b]).unwrap_or("?"));
    }
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
