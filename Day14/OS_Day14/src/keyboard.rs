//! Jour 14 — Driver clavier avec queue de scancodes
//! Pas d'allocation dynamique — buffer circulaire fixe

use crate::serial;

const QUEUE_SIZE: usize = 64;

/// Queue circulaire de scancodes — remplie par l'IRQ, vidée par le poll
pub struct ScancodeQueue {
    buffer: [u8; QUEUE_SIZE],
    head: usize, // prochain à lire
    tail: usize, // prochain à écrire
    count: usize,
}

impl ScancodeQueue {
    pub const fn new() -> Self {
        ScancodeQueue {
            buffer: [0; QUEUE_SIZE],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Ajoute un scancode — appelé depuis le handler d'interruption
    pub fn push(&mut self, scancode: u8) -> bool {
        if self.count >= QUEUE_SIZE {
            return false; // queue pleine — scancode perdu
        }
        self.buffer[self.tail] = scancode;
        self.tail = (self.tail + 1) % QUEUE_SIZE;
        self.count += 1;
        true
    }

    /// Retire un scancode — appelé depuis le poll de la "tâche async"
    pub fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            return None;
        }
        let val = self.buffer[self.head];
        self.head = (self.head + 1) % QUEUE_SIZE;
        self.count -= 1;
        Some(val)
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

pub static mut SCANCODE_QUEUE: ScancodeQueue = ScancodeQueue::new();

/// Appelé depuis le handler IRQ1 — lit le port clavier et empile
pub fn handle_keyboard_interrupt() {
    let scancode = unsafe {
        let val: u8;
        core::arch::asm!("in al, dx", out("al") val, in("dx") 0x60u16);
        val
    };

    unsafe {
        if !(*core::ptr::addr_of_mut!(SCANCODE_QUEUE)).push(scancode) {
            serial::println("  ATTENTION : queue clavier pleine !");
        }
    }
}

/// Table de conversion scancode -> caractère ASCII (set 1, touches principales)
fn scancode_to_ascii(scancode: u8) -> Option<char> {
    // On ignore les "key release" (bit 7 = 1)
    if scancode & 0x80 != 0 {
        return None;
    }
    match scancode {
        0x1E => Some('a'), 0x30 => Some('b'), 0x2E => Some('c'), 0x20 => Some('d'),
        0x12 => Some('e'), 0x21 => Some('f'), 0x22 => Some('g'), 0x23 => Some('h'),
        0x17 => Some('i'), 0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'),
        0x32 => Some('m'), 0x31 => Some('n'), 0x18 => Some('o'), 0x19 => Some('p'),
        0x10 => Some('q'), 0x13 => Some('r'), 0x1F => Some('s'), 0x14 => Some('t'),
        0x16 => Some('u'), 0x2F => Some('v'), 0x11 => Some('w'), 0x2D => Some('x'),
        0x15 => Some('y'), 0x2C => Some('z'),
        0x39 => Some(' '), // espace
        0x1C => Some('\n'), // entree
        _ => None,
    }
}

/// "Future" simple — pas un vrai Future async/await complet
/// mais le même principe : poll() qui retourne Some/None
pub struct KeyboardFuture;

impl KeyboardFuture {
    /// Poll non bloquant — retourne le prochain caractère si disponible
    pub fn poll(&self) -> Option<char> {
        loop {
            let scancode = unsafe {
                (*core::ptr::addr_of_mut!(SCANCODE_QUEUE)).pop()
            };
            match scancode {
                Some(sc) => {
                    if let Some(ch) = scancode_to_ascii(sc) {
                        return Some(ch);
                    }
                    // scancode ignoré (release ou touche non geree) — continuer
                }
                None => return None, // queue vide
            }
        }
    }
}
