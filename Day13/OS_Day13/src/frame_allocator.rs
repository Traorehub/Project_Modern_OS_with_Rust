//! Jour 11 — Allocateur de frames physiques
//! Une frame = une huge page de 2Mo dans notre cas
//! On gère les 16 frames de notre mapping 32Mo

use crate::serial;

/// Taille d'une frame (huge page 2Mo)
pub const FRAME_SIZE: u64 = 2 * 1024 * 1024; // 2Mo

/// Nombre total de frames disponibles
/// Notre mapping : 32Mo / 2Mo = 16 frames
pub const FRAME_COUNT: usize = 16;

/// Adresse de début du mapping
pub const MAPPING_START: u64 = 0x0;

/// État d'une frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameState {
    Free,
    Used,
    Reserved, // kernel, tables de pages, etc.
}

/// Allocateur de frames — bitmap simple
pub struct FrameAllocator {
    frames: [FrameState; FRAME_COUNT],
    free_count: usize,
}

impl FrameAllocator {
    /// Crée un nouvel allocateur
    pub const fn new() -> Self {
        FrameAllocator {
            frames: [FrameState::Free; FRAME_COUNT],
            free_count: FRAME_COUNT,
        }
    }

    /// Initialise l'allocateur en marquant les frames utilisées
    pub fn init(&mut self) {
        // Frame 0 (0x000000 - 0x1fffff) : zone basse, réservée
        self.reserve_frame(0);

        // Frame 1 (0x200000 - 0x3fffff) : kernel, réservée
        self.reserve_frame(1);

        // Frame 2 (0x400000 - 0x5fffff) : tables de pages (0x70000), réservée
        self.reserve_frame(2);

        // Frames 3-15 : libres pour allocation
        serial::println("  Allocateur initialise");
        self.print_stats();
    }

    /// Réserve une frame spécifique
    fn reserve_frame(&mut self, index: usize) {
        if index < FRAME_COUNT && self.frames[index] == FrameState::Free {
            self.frames[index] = FrameState::Reserved;
            self.free_count -= 1;
        }
    }

    /// Alloue une frame libre — retourne son adresse physique
    pub fn alloc(&mut self) -> Option<u64> {
        for i in 0..FRAME_COUNT {
            if self.frames[i] == FrameState::Free {
                self.frames[i] = FrameState::Used;
                self.free_count -= 1;
                let phys_addr = MAPPING_START + (i as u64 * FRAME_SIZE);
                return Some(phys_addr);
            }
        }
        None // plus de frames disponibles
    }

    /// Libère une frame par son adresse physique
    pub fn free(&mut self, phys_addr: u64) -> bool {
        let index = ((phys_addr - MAPPING_START) / FRAME_SIZE) as usize;
        if index >= FRAME_COUNT {
            return false; // adresse invalide
        }
        if self.frames[index] == FrameState::Reserved {
            return false; // impossible de libérer une frame réservée
        }
        if self.frames[index] == FrameState::Used {
            self.frames[index] = FrameState::Free;
            self.free_count += 1;
            return true;
        }
        false // déjà libre
    }

    /// Retourne le nombre de frames libres
    pub fn free_count(&self) -> usize {
        self.free_count
    }

    /// Retourne le nombre total de frames
    pub fn total_count(&self) -> usize {
        FRAME_COUNT
    }

    /// Affiche les statistiques
    pub fn print_stats(&self) {
        serial::print("  Frames : ");
        print_num(self.free_count);
        serial::print(" libres / ");
        print_num(FRAME_COUNT);
        serial::println(" total");
    }

    /// Affiche la carte mémoire complète
    pub fn print_map(&self) {
        serial::println("\n=== Carte memoire (frames 2Mo) ===");
        for i in 0..FRAME_COUNT {
            let addr = MAPPING_START + (i as u64 * FRAME_SIZE);
            serial::print("  Frame ");
            print_num(i);
            serial::print(" [");
            print_hex(addr);
            serial::print(" - ");
            print_hex(addr + FRAME_SIZE - 1);
            serial::print("] : ");
            match self.frames[i] {
                FrameState::Free     => serial::println("LIBRE"),
                FrameState::Used     => serial::println("UTILISE"),
                FrameState::Reserved => serial::println("RESERVE"),
            }
        }
        serial::println("=== Fin carte memoire ===\n");
    }
}

/// Allocateur global statique
pub static mut ALLOCATOR: FrameAllocator = FrameAllocator::new();

/// Initialise l'allocateur global
pub fn init() {
    unsafe { (*core::ptr::addr_of_mut!(ALLOCATOR)).init(); }
}

/// Alloue une frame depuis l'allocateur global
pub fn alloc_frame() -> Option<u64> {
    unsafe { (*core::ptr::addr_of_mut!(ALLOCATOR)).alloc() }
}

/// Libère une frame dans l'allocateur global
pub fn free_frame(addr: u64) -> bool {
    unsafe { (*core::ptr::addr_of_mut!(ALLOCATOR)).free(addr) }
}

// ─── Utilitaires d'affichage ─────────────────────────────────────────────────

fn print_hex(v: u64) {
    serial::print("0x");
    let mut buf = [0u8; 16];
    let mut i = 16usize;
    let mut n = v;
    if n == 0 { serial::print("0"); return; }
    while n > 0 && i > 0 {
        i -= 1;
        let nibble = (n & 0xF) as u8;
        buf[i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        n >>= 4;
    }
    for &b in &buf[i..] {
        serial::print(core::str::from_utf8(&[b]).unwrap_or("?"));
    }
}

fn print_num(n: usize) {
    if n == 0 { serial::print("0"); return; }
    let mut buf = [0u8; 20];
    let mut i = 20usize;
    let mut v = n;
    while v > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &b in &buf[i..] {
        serial::print(core::str::from_utf8(&[b]).unwrap_or("?"));
    }
}
