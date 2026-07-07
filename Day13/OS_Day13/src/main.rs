//! Jour 13 — Heap Allocator : Linked List
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

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 13 - Heap Allocator : Linked List");
    vga::WRITER.lock().write_fmt(format_args!("Jour 13 - Heap Allocator\n")).ok();
    serial::println("=======================================");
    vga::WRITER.lock().write_fmt(format_args!("=======================================\n\n")).ok();

    gdt::init();
    idt::init();

    // Init frame allocator + heap mapping
    frame_allocator::init();
    heap::map_heap();

    // Init heap allocator
    heap_allocator::init();

    // Afficher état initial
    unsafe { (*core::ptr::addr_of_mut!(heap_allocator::HEAP_ALLOC)).print_map(); }

    // Test 1 : allocation simple
    serial::println("Test 1 : allocation de 3 blocs");
    let p1 = heap_allocator::alloc(64);
    let p2 = heap_allocator::alloc(128);
    let p3 = heap_allocator::alloc(256);

    match p1 { Some(p) => { serial::print("  p1="); print_hex(p as u64); serial::println(" (64o)"); } None => serial::println("  p1=None") }
    match p2 { Some(p) => { serial::print("  p2="); print_hex(p as u64); serial::println(" (128o)"); } None => serial::println("  p2=None") }
    match p3 { Some(p) => { serial::print("  p3="); print_hex(p as u64); serial::println(" (256o)"); } None => serial::println("  p3=None") }

    // Test 2 : écriture dans les blocs alloués
    serial::println("\nTest 2 : ecriture dans les blocs");
    if let Some(ptr) = p1 {
        unsafe {
            core::ptr::write_volatile(ptr as *mut u64, 0xAAAA_BBBB_CCCC_DDDD);
            let val = core::ptr::read_volatile(ptr as *const u64);
            serial::print("  p1 -> ecriture/lecture : ");
            if val == 0xAAAA_BBBB_CCCC_DDDD { serial::println("OK"); } else { serial::println("ERREUR"); }
        }
    }

    // Afficher état après allocations
    unsafe { (*core::ptr::addr_of_mut!(heap_allocator::HEAP_ALLOC)).print_map(); }

    // Test 3 : libération et coalescence
    serial::println("Test 3 : liberation + coalescence");
    if let Some(p) = p1 { heap_allocator::free(p); serial::println("  p1 libere"); }
    if let Some(p) = p2 { heap_allocator::free(p); serial::println("  p2 libere"); }

    unsafe { (*core::ptr::addr_of_mut!(heap_allocator::HEAP_ALLOC)).print_map(); }

    // Test 4 : double-free détecté
    serial::println("Test 4 : double-free");
    if let Some(p) = p1 {
        let ok = heap_allocator::free(p);
        if !ok { serial::println("  Double-free detecte et refuse ✓"); }
    }

    // Test 5 : use-after-free démonstration
    serial::println("\nTest 5 : use-after-free (demonstration)");
    let p4 = heap_allocator::alloc(64);
    if let Some(ptr) = p4 {
        unsafe { core::ptr::write_volatile(ptr as *mut u64, 0x1111_2222_3333_4444); }
        heap_allocator::free(ptr);
        // UAF : lire après free — la mémoire appartient maintenant à l'allocateur
        unsafe {
            let val = core::ptr::read_volatile(ptr as *const u64);
            serial::print("  Valeur apres free (UAF) : ");
            print_hex(val);
            serial::println(" (dangereux en C, interdit en Rust safe)");
        }
    }

    // Stats finales
    let (free_b, used_b, blocs) = unsafe {
        (*core::ptr::addr_of_mut!(heap_allocator::HEAP_ALLOC)).stats()
    };
    serial::println("\n=== Stats finales ===");
    serial::print("  Blocs : "); print_num(blocs); serial::println("");
    serial::print("  Libre : "); print_num(free_b); serial::println(" octets");
    serial::print("  Utilise : "); print_num(used_b); serial::println(" octets");

    serial::println("\nHeap Allocator OK.");
    vga::WRITER.lock().write_fmt(format_args!("\nHeap Allocator OK.\n")).ok();

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
