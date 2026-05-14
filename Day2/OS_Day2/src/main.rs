#![no_std] // pas de bibliothèque standard
#![no_main] // pas de fonction main() classique

use core::panic::PanicInfo;

// Adresse du port série COM1 - toujours 0x3F8 sur x86
const COM1: u16 = 0x3F8;

// Écrire un octet sur le port série
unsafe fn write_serial(byte: u8) {
    // "out" est une instruction x86 qui envoie
    // une valeur sur un port hardware
    core::arch::asm!(
        "out dx, al",
        in("dx") COM1,
        in("al") byte,
    );
}

// Écrire un string entier
unsafe fn print_serial(s: &str) {
    for byte in s.bytes() {
        write_serial(byte);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        print_serial("Hello from my OS!\n");
    }
    loop {}
}

#[panic_handler]
// Le "-> !" signifie que cette fonction ne retourne JAMAIS
// Normal : un OS ne "termine" pas
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
