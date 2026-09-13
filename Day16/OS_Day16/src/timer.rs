//! Jour 16 — Timer PIT 8254 : le pouls du noyau
//! IRQ0 periodique -> compteur de ticks -> base du scheduling preemptif

use core::sync::atomic::{AtomicU64, Ordering};
use crate::serial;

/// Frequence de base du PIT 8254 — gravee dans le silicium depuis l'IBM PC
const PIT_BASE_FREQUENCY: u32 = 1_193_182;

/// Nombre d'interruptions IRQ0 par seconde (1 tick = 10 ms)
pub const TICKS_PER_SECOND: u32 = 100;

const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0_DATA: u16 = 0x40;

/// Canal 0, acces lobyte/hibyte, mode 3 (onde carree), compteur binaire
const PIT_MODE3_SQUARE_WAVE: u8 = 0x36;

/// Compteur global incremente par le handler IRQ0.
/// Atomique car ecrit depuis une interruption et lu depuis la boucle principale.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val);
}

/// Programme le canal 0 du PIT pour declencher IRQ0 a `TICKS_PER_SECOND` Hz.
///
/// Le PIT decremente un compteur a chaque oscillation de sa frequence de base.
/// Quand il atteint zero il leve IRQ0 puis recharge le diviseur.
pub unsafe fn init() {
    let divisor = (PIT_BASE_FREQUENCY / TICKS_PER_SECOND) as u16;

    outb(PIT_COMMAND, PIT_MODE3_SQUARE_WAVE);
    outb(PIT_CHANNEL0_DATA, (divisor & 0xFF) as u8); // octet bas d'abord
    outb(PIT_CHANNEL0_DATA, (divisor >> 8) as u8);   // puis octet haut

    serial::print("  PIT programme : diviseur ");
    serial::print_dec(divisor as u64);
    serial::print(" -> ");
    serial::print_dec(TICKS_PER_SECOND as u64);
    serial::print(" Hz (1 tick = ");
    serial::print_dec((1000 / TICKS_PER_SECOND) as u64);
    serial::println(" ms)");
}

/// Appele depuis le handler IRQ0 — doit rester le plus court possible.
/// Aucun verrou pris ici : prendre le Mutex VGA depuis une interruption
/// provoquerait un deadlock si la boucle principale le detient deja.
pub fn on_tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn uptime_seconds() -> u64 {
    ticks() / TICKS_PER_SECOND as u64
}

pub fn uptime_ms() -> u64 {
    ticks() * (1000 / TICKS_PER_SECOND as u64)
}
