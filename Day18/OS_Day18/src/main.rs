//! Jour 18 : ordonnanceur preemptive, schedule() depuis IRQ0
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
mod context;
mod scheduler;

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

fn ok_label(label: &str) {
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("{}", label)).ok();
    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("======================================\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("  JOUR 18 - SCHEDULER PREEMPTIF\n")).ok();
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    serial::println("======================================");
    serial::println("  JOUR 18 - SCHEDULER PREEMPTIF");
    serial::println("======================================\n");

    serial::println("[1/6] Securite CPU : GDT + IDT + IST...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[1/6] Securite CPU : ")).ok();

    gdt::init();
    idt::init();

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    serial::println("      GDT/IDT/IST operationnels (Jours 7-8)\n");

    serial::println("[2/6] Memoire : pagination + allocateur + heap...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[2/6] Memoire : ")).ok();

    frame_allocator::init();
    let heap_ok = heap::map_heap();
    heap_allocator::init();

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

    serial::println("[3/6] Ecran : VGA Safe Wrapper...");
    ok_label("[3/6] Ecran   : ");
    serial::println("      VGA Writer + Mutex operationnel (Jours 4-5)\n");

    serial::println("[4/6] Timer : PIT 8254 canal 0...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[4/6] Timer   : ")).ok();

    unsafe { timer::init(); }

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK (")).ok();
    vga::WRITER.lock().write_dec(timer::TICKS_PER_SECOND as u64);
    vga::WRITER.lock().write_fmt(format_args!(" Hz)\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      PIT programme, IRQ0 des le sti (Jour 16)\n");

    serial::println("[5/6] Interruptions : PIC 8259 + IRQ0 + IRQ1...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[5/6] IRQ     : ")).ok();

    unsafe { pic::init(); }
    unsafe { core::arch::asm!("sti"); }

    vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("OK\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
    serial::println("      PIC remappe, interruptions activees (Jours 14/16)\n");

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
        vga::WRITER.lock().write_fmt(format_args!(" ticks)\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
        serial::print("  OK - ");
        serial::print_dec(timer::ticks());
        serial::println(" tick(s) recus\n");
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("ECHEC\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);
        serial::println("  ECHEC : aucun tick recu.\n");
    }

    serial::println("[6/6] Scheduler : A et B bouclent sans yield...");
    vga::WRITER.lock().set_color(vga2::Color::Cyan, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("[6/6] Preempt : ")).ok();

    scheduler::init();
    serial::println("  taches A et B armees (boucle infinie, aucun switch)");
    serial::println("  attente 2 s : IRQ0 doit les couper de force...");

    let start = timer::uptime_seconds();
    while timer::uptime_seconds().saturating_sub(start) < 2 {
        unsafe { core::arch::asm!("hlt"); }
    }

    let a = scheduler::counter_a();
    let b = scheduler::counter_b();
    let sw = scheduler::switches();
    let preempt_ok = a > 0 && b > 0 && sw >= 2;

    if preempt_ok {
        vga::WRITER.lock().set_color(vga2::Color::LightGreen, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("OK (A=")).ok();
        vga::WRITER.lock().write_dec(a);
        vga::WRITER.lock().write_fmt(format_args!(" B=")).ok();
        vga::WRITER.lock().write_dec(b);
        vga::WRITER.lock().write_fmt(format_args!(")\n\n")).ok();
        serial::print("  OK - A=");
        serial::print_dec(a);
        serial::print(" B=");
        serial::print_dec(b);
        serial::print(" switches=");
        serial::print_dec(sw);
        serial::println(" (aucun yield volontaire)\n");
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("ECHEC\n\n")).ok();
        serial::print("  ECHEC : A=");
        serial::print_dec(a);
        serial::print(" B=");
        serial::print_dec(b);
        serial::print(" switches=");
        serial::print_dec(sw);
        serial::println(" (attendu A>0, B>0, switches>=2)\n");
    }

    vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    vga::WRITER.lock().write_fmt(format_args!("======================================\n")).ok();
    if timer_irq_ok && preempt_ok {
        vga::WRITER.lock().write_fmt(format_args!("  PREEMPTIF : OK\n")).ok();
    } else {
        vga::WRITER.lock().set_color(vga2::Color::LightRed, vga2::Color::Black);
        vga::WRITER.lock().write_fmt(format_args!("  DEMO PARTIELLE\n")).ok();
        vga::WRITER.lock().set_color(vga2::Color::Yellow, vga2::Color::Black);
    }
    vga::WRITER.lock().write_fmt(format_args!("======================================\n\n")).ok();
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Black);

    serial::println("======================================");
    serial::println("  NOYAU JOUR 18 : scheduler preemptif");
    serial::println("======================================\n");
    serial::println("A et B continuent d'etre coupes par IRQ0. Tapez au clavier.\n");

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
