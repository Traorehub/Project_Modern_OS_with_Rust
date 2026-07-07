//! Jour 14 — PIC 8259 : reprogrammation pour les interruptions matérielles

use crate::serial;

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA:    u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA:    u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

/// Offsets pour remapper les IRQ — éviter le conflit avec les exceptions CPU 0-31
pub const PIC1_OFFSET: u8 = 32; // IRQ0 -> interruption 32
pub const PIC2_OFFSET: u8 = 40; // IRQ8 -> interruption 40

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val);
}

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    val
}

/// Reinitialise et remap le PIC
pub unsafe fn init() {
    // Sauvegarder les masques actuels
    let mask1 = inb(PIC1_DATA);
    let mask2 = inb(PIC2_DATA);

    // ICW1 : initialisation
    outb(PIC1_COMMAND, 0x11);
    outb(PIC2_COMMAND, 0x11);

    // ICW2 : offsets
    outb(PIC1_DATA, PIC1_OFFSET);
    outb(PIC2_DATA, PIC2_OFFSET);

    // ICW3 : cascade
    outb(PIC1_DATA, 4);
    outb(PIC2_DATA, 2);

    // ICW4 : mode 8086
    outb(PIC1_DATA, 0x01);
    outb(PIC2_DATA, 0x01);

    // Restaurer les masques (tout masqué sauf clavier = IRQ1)
    outb(PIC1_DATA, 0xFD); // 0xFD = 11111101 -> seul IRQ1 (clavier) actif
    outb(PIC2_DATA, 0xFF); // tout masqué sur PIC2

    serial::println("  PIC remappe : IRQ0-7 -> 32-39, IRQ8-15 -> 40-47");
}

/// Envoie End-Of-Interrupt après traitement d'une IRQ
pub unsafe fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}
