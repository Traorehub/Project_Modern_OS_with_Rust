//! IDT — Interrupt Descriptor Table
//! Jour 7 : gestion des exceptions CPU en 64 bits

use core::ptr::addr_of_mut;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::serial;

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        let idt = &mut *addr_of_mut!(IDT);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
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
    frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial::println("\n[EXCEPTION] Double Fault !");
    serial::print("  RIP : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    panic!("double fault");
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
    frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let cr2 = x86_64::registers::control::Cr2::read_raw();
    serial::println("\n[EXCEPTION] Page Fault !");
    serial::print("  Adresse accedee : ");
    print_hex(cr2);
    serial::println("");
    serial::print("  RIP             : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    serial::print("  Error code      : ");
    print_hex(error_code.bits());
    serial::println("");
    if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        serial::println("  Cause : violation de protection");
    } else {
        serial::println("  Cause : page non presente");
    }
    if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        serial::println("  Acces : ecriture");
    } else {
        serial::println("  Acces : lecture");
    }
    panic!("page fault");
}

fn print_hex(value: u64) {
    serial::print("0x");
    let mut buf = [0u8; 16];
    let mut i = 16;
    let mut v = value;
    if v == 0 {
        serial::print("0");
        return;
    }
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
