#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;

use core::panic::PanicInfo;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    // Test 1 : port série uniquement, sans VGA
    serial::println("64-bit kernel alive!");
    serial::println("Testing VGA...");

    // Test 2 : VGA direct sans Mutex
    unsafe {
        vga::write_char(0, 0, b'6', 0x0F);
        vga::write_char(0, 1, b'4', 0x0F);
        vga::write_str(1, 0, "Hello from 64-bit!", 0x0F);
    }

    serial::println("VGA OK");
    serial::println("Switch 64-bit complete!");

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("KERNEL PANIC");
    if let Some(msg) = info.message().as_str() {
        serial::print("Reason: ");
        serial::println(msg);
    }
    unsafe { halt(); }
}
