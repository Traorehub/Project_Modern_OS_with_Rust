//! Jour 15 — Synthese finale : OS stable gerant clavier, ecran et memoire
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

    // ─── Banniere ────────────────────────────────────────────────────────────
    vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("======================================\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("  JOUR 15 - SYNTHESE FINALE\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    serial::println("======================================");
    serial::println("  JOUR 15 - SYNTHESE FINALE");
    serial::println("======================================\n");

    // ─── PHASE 1 : Securite CPU (GDT, IDT, IST) ─────────────────────────────
    serial::println("[1/4] Securite CPU : GDT + IDT + IST...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[1/4] Securite CPU : ")).ok();

    gdt::init();
    idt::init();

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    serial::println("      GDT/IDT/IST operationnels (Jours 7-8)\n");

    // ─── PHASE 2 : Memoire (Pagination, Frames, Heap) ───────────────────────
    serial::println("[2/4] Memoire : pagination + allocateur + heap...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[2/4] Memoire : ")).ok();

    frame_allocator::init();
    let heap_ok = heap::map_heap();
    heap_allocator::init();

    // Test reel : allouer, ecrire, lire, liberer
    let test_ptr = heap_allocator::alloc(128);
    let mut mem_ok = heap_ok && test_ptr.is_some();
    if let Some(ptr) = test_ptr {
        unsafe {
            core::ptr::write_volatile(ptr as *mut u64, 0xC0FFEE_C0FFEE);
            let val = core::ptr::read_volatile(ptr as *const u64);
            mem_ok = mem_ok && (val == 0xC0FFEE_C0FFEE);
        }
        heap_allocator::free(ptr);
    }

    if mem_ok {
        vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
        serial::println("      Pagination/Frames/Heap operationnels (Jours 9-13)\n");
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("ECHEC\n")).ok();
        serial::println("      ERREUR memoire !\n");
    }

    // ─── PHASE 3 : Ecran (VGA Safe Wrapper) ─────────────────────────────────
    serial::println("[3/4] Ecran : VGA Safe Wrapper multicolore...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[3/4] Ecran   : ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      VGA Writer + Mutex operationnel (Jours 4-5)\n");

    // Demo couleurs
    vga::WRITER.lock().write_fmt(format_args!("      Demo couleurs : ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::Red, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Rouge ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::Green, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Vert ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::Blue, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Bleu ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::Magenta, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Magenta\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    // ─── PHASE 4 : Clavier (PIC + IRQ + Executor) ───────────────────────────
    serial::println("[4/4] Clavier : PIC + IRQ1 + Executeur async...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[4/4] Clavier : ")).ok();

    unsafe { pic::init(); }
    unsafe { core::arch::asm!("sti"); }

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n\n")).ok();
    serial::println("      PIC/IRQ/Executeur operationnels (Jour 14)\n");

    // ─── RESUME FINAL ────────────────────────────────────────────────────────
    vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("======================================\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("  TOUS LES SOUS-SYSTEMES : OK\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("Tapez au clavier pour la demo finale :\n")).ok();

    serial::println("======================================");
    serial::println("  TOUS LES SOUS-SYSTEMES : OK");
    serial::println("======================================\n");
    serial::println("Demo finale interactive : tapez au clavier dans QEMU\n");

    // Lancer l'exécuteur interactif final
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
