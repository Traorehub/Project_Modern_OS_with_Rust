//! Jour 16 — Timer PIT 8254 : le noyau acquiert une notion du temps
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
mod timer;
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
    vga::WRITER.lock().write_fmt(format_args!("  JOUR 16 - TIMER PIT 8254 (IRQ0)\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    serial::println("======================================");
    serial::println("  JOUR 16 - TIMER PIT 8254 (IRQ0)");
    serial::println("======================================\n");

    // ─── PHASE 1 : Securite CPU (GDT, IDT, IST) ─────────────────────────────
    serial::println("[1/5] Securite CPU : GDT + IDT + IST...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[1/5] Securite CPU : ")).ok();

    gdt::init();
    idt::init();

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    serial::println("      GDT/IDT/IST operationnels (Jours 7-8)\n");

    // ─── PHASE 2 : Memoire (Pagination, Frames, Heap) ───────────────────────
    serial::println("[2/5] Memoire : pagination + allocateur + heap...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[2/5] Memoire : ")).ok();

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
    serial::println("[3/5] Ecran : VGA Safe Wrapper multicolore...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[3/5] Ecran   : ")).ok();
    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      VGA Writer + Mutex operationnel (Jours 4-5)\n");

    // ─── PHASE 4 : Timer PIT (NOUVEAU Jour 16) ──────────────────────────────
    serial::println("[4/5] Timer : programmation du PIT 8254 canal 0...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[4/5] Timer   : ")).ok();

    unsafe { timer::init(); }

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK (")).ok();
    vga::WRITER.lock().write_dec(timer::TICKS_PER_SECOND as u64);
    vga::WRITER.lock().write_fmt(format_args!(" Hz)\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      PIT programme — IRQ0 arrivera des le sti (Jour 16)\n");

    // ─── PHASE 5 : Interruptions materielles (PIC + sti) ────────────────────
    serial::println("[5/5] Interruptions : PIC 8259 + IRQ0 + IRQ1...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[5/5] IRQ     : ")).ok();

    unsafe { pic::init(); }
    unsafe { core::arch::asm!("sti"); }

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      PIC remappe, interruptions activees (Jours 14/16)\n");

    // ─── VERIFICATION : IRQ0 arrive-t-elle vraiment ? ───────────────────────
    //
    // C'est LE test du Jour 16 : sans toucher au clavier, le compteur de ticks
    // doit passer de 0 a une valeur non nulle. Boucle bornee volontairement :
    // si l'EOI ou le masque PIC est faux, on affiche ECHEC au lieu de figer.
    serial::println("Verification IRQ0 : attente du premier tick...");
    vga::WRITER.lock().write_fmt(format_args!("Verification IRQ0 : ")).ok();

    let mut spins: u64 = 0;
    let timer_irq_ok = loop {
        if timer::ticks() > 0 {
            break true;
        }
        spins += 1;
        if spins > 200_000_000 {
            break false;
        }
        core::hint::spin_loop();
    };

    if timer_irq_ok {
        vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("OK (")).ok();
        vga::WRITER.lock().write_dec(timer::ticks());
        vga::WRITER.lock().write_fmt(format_args!(" ticks)\n\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

        serial::print("  OK - ");
        serial::print_dec(timer::ticks());
        serial::println(" tick(s) recus sans interaction clavier\n");
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("ECHEC\n\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
        serial::println("  ECHEC — aucun tick recu.");
        serial::println("  Verifier : masque PIC1 = 0xFC, idt[32] enregistre, send_eoi(0) present.\n");
    }

    // ─── RESUME FINAL ────────────────────────────────────────────────────────
    vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("======================================\n")).ok();
    if timer_irq_ok {
        vga::WRITER.lock().write_fmt(format_args!("  NOYAU CADENCE : ")).ok();
        vga::WRITER.lock().write_dec(timer::TICKS_PER_SECOND as u64);
        vga::WRITER.lock().write_fmt(format_args!(" Hz\n")).ok();
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("  TIMER NON FONCTIONNEL\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    }
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    serial::println("======================================");
    serial::println("  NOYAU CADENCE — timer + clavier actifs");
    serial::println("======================================\n");
    serial::println("L'uptime defile seul. Tapez au clavier en parallele.\n");

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
