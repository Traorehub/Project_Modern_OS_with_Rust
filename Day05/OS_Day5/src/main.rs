//! Jour 5 - Safe Wrapper VGA + println!
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;

use core::panic::PanicInfo;
use vga2::Color;

const COM1: u16 = 0x3F8;

unsafe fn write_serial(byte: u8) {
    core::arch::asm!("out dx, al", in("dx") COM1, in("al") byte);
}

unsafe fn print_serial(s: &str) {
    for b in s.bytes() { unsafe { write_serial(b); } }
}

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    unsafe { print_serial("Entering _start\n"); }

    // Initialisation de l'écran
    unsafe { print_serial("Calling WRITER.lock().clear()\n"); }
    vga2::WRITER.lock().clear();
    unsafe { print_serial("WRITER.lock().clear() done\n"); }

    // Démonstration de println! - plus d'unsafe visible !
    println!("Jour 5 - Safe VGA Wrapper");
    println!("==========================");
    println!();

    // Changement de couleur via le Writer
    vga2::WRITER.lock().set_color(Color::Cyan, Color::Black);
    println!("Affichage en cyan !");

    vga2::WRITER.lock().set_color(Color::Yellow, Color::Black);
    println!("Affichage en jaune !");

    vga2::WRITER.lock().set_color(Color::White, Color::Black);
    println!("Retour en blanc.");
    println!();

    // Démonstration du formatage
    let x: u32 = 42;
    let y: u32 = 1337;
    println!("Valeur de x = {}", x);
    println!("Valeur de y = {}", y);
    println!("x + y = {}", x + y);
    println!();

    // Démonstration du scroll automatique
    println!("Test du scroll automatique :");
    for i in 0..30 {
        println!("  Ligne {} - le scroll fonctionne", i);
    }

    // Confirmation sur port série
    unsafe { print_serial("Jour 5 OK - Safe VGA wrapper operationnel\n"); }

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Panic handler affiche sur VGA en rouge vif
    vga2::WRITER.lock().set_color(Color::White, Color::Red);
    println!("\n=== KERNEL PANIC ===");

    if let Some(msg) = info.message().as_str() {
        println!("Raison  : {}", msg);
    }
    if let Some(loc) = info.location() {
        println!("Fichier : {}", loc.file());
        println!("Ligne   : {}", loc.line());
    }
    println!("Systeme arrete.");

    unsafe { halt(); }
}
