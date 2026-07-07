//! IDT — Jour 8 : Double Fault avec IST

use core::ptr::addr_of_mut;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::{serial, gdt, vga, vga2, keyboard, pic};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        let idt = &mut *addr_of_mut!(IDT);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        // Double Fault avec IST — stack dédiée index 0
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

        // IRQ1 (clavier) remappee a l'interruption 33 (32 + 1)
        idt[33].set_handler_fn(keyboard_interrupt_handler);

        idt.load();
    }
}

extern "x86-interrupt" fn divide_error_handler(frame: InterruptStackFrame) {
    serial::println("\n[EXCEPTION] Division par zero !");
    serial::print("  RIP : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    panic!("divide error");
}

extern "x86-interrupt" fn invalid_opcode_handler(frame: InterruptStackFrame) {
    serial::println("\n[EXCEPTION] Opcode invalide !");
    serial::print("  RIP : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    panic!("invalid opcode");
}

extern "x86-interrupt" fn double_fault_handler(
    _frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    // Handler ASM pur — pas de prologue Rust
    unsafe extern "C" { fn double_fault_handler_asm() -> !; }
    unsafe { double_fault_handler_asm() }
}

extern "x86-interrupt" fn gpf_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    serial::println("\n[EXCEPTION] General Protection Fault !");
    serial::print("  RIP        : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    serial::print("  Error code : ");
    print_hex(error_code);
    serial::println("");
    panic!("general protection fault");
}

extern "x86-interrupt" fn page_fault_handler(
    _frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    serial::println("\n[PAGE FAULT] Corruption stack -> Double Fault...");
    // Corrompre RSP → toute exception suivante → Double Fault
    unsafe {
        core::arch::asm!(
            "mov rsp, 0x5000000",
            "xor rax, rax",
            "div rax",
            options(nostack, noreturn)
        );
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: InterruptStackFrame) {
    keyboard::handle_keyboard_interrupt();
    unsafe { pic::send_eoi(1); } // IRQ1
}

fn print_hex(value: u64) {
    serial::print("0x");
    let mut buf = [0u8; 16];
    let mut i = 16;
    let mut v = value;
    if v == 0 { serial::print("0"); return; }
    while v > 0 && i > 0 {
        i -= 1;
        let nibble = (v & 0xF) as u8;
        buf[i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        v >>= 4;
    }
    for &b in &buf[i..] {
        serial::print(core::str::from_utf8(&[b]).unwrap_or("?"));
    }
}
