#![no_std]
#![no_main]

use core::panic::PanicInfo;

const COM1: u16 = 0x3F8;

unsafe fn write_serial(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") COM1,
            in("al") byte,
        );
    }
}

unsafe fn print_serial(s: &str) {
    for byte in s.bytes() {
        unsafe { write_serial(byte); }
    }
}

unsafe fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    unsafe {
        print_serial("OS starting...\n");
        panic!("Test panic : quelque chose a mal tourne !");
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        print_serial("\n=== KERNEL PANIC ===\n");
        if let Some(msg) = info.message().as_str() {
            print_serial("Raison  : ");
            print_serial(msg);
            print_serial("\n");
        }
        if let Some(location) = info.location() {
            print_serial("Fichier : ");
            print_serial(location.file());
            print_serial("\n");
        }
        print_serial("Systeme arrete.\n");
        print_serial("====================\n");
        halt();
    }
}
