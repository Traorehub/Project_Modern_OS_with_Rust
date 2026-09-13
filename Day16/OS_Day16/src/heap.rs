//! Jour 12 — Mapping du heap
//! On mappe une zone virtuelle dédiée en allouant des frames physiques

use crate::{serial, frame_allocator, paging};

/// Adresse virtuelle de début du heap
pub const HEAP_START: u64 = 0x4000000; // 64Mo

/// Taille du heap : 4 frames de 2Mo = 8Mo
pub const HEAP_SIZE: u64 = 4 * 2 * 1024 * 1024; // 8Mo

/// Adresse virtuelle de fin du heap
pub const HEAP_END: u64 = HEAP_START + HEAP_SIZE;

/// Mappe le heap en allouant des frames physiques
/// et en les insérant dans les tables de pages
pub fn map_heap() -> bool {
    serial::println("\n=== Mapping du heap ===");
    serial::print("  Zone virtuelle : ");
    print_hex(HEAP_START);
    serial::print(" - ");
    print_hex(HEAP_END - 1);
    serial::println("");

    let cr3 = paging::read_cr3();
    let pml4_addr = cr3 & !0xFFF;

    // Notre heap est à 0x4000000 = PML4[0] → PDPT[0] → PD[2..5]
    // PD[2] = 0x4000000 / 2Mo = index 32
    // PD[3] = index 33, etc.

    let heap_pd_start = (HEAP_START / (2 * 1024 * 1024)) as usize;

    // Récupérer l'adresse de la PD depuis les tables existantes
    let pml4_entry = unsafe {
        core::ptr::read_volatile(pml4_addr as *const u64)
    };
    if pml4_entry & 1 == 0 {
        serial::println("  ERREUR : PML4[0] non present");
        return false;
    }

    let pdpt_addr = pml4_entry & 0x000FFFFF_FFFFF000;
    let pdpt_entry = unsafe {
        core::ptr::read_volatile(pdpt_addr as *const u64)
    };
    if pdpt_entry & 1 == 0 {
        serial::println("  ERREUR : PDPT[0] non present");
        return false;
    }

    let pd_addr = pdpt_entry & 0x000FFFFF_FFFFF000;

    // Allouer et mapper 4 frames pour le heap
    let mut mapped = 0;
    for i in 0..4usize {
        let pd_index = heap_pd_start + i;
        let pd_entry_addr = pd_addr + (pd_index * 8) as u64;

        // Vérifier que la PD entry est vide
        let existing = unsafe {
            core::ptr::read_volatile(pd_entry_addr as *const u64)
        };
        if existing & 1 != 0 {
            serial::print("  PD[");
            print_num(pd_index);
            serial::println("] deja utilise — skip");
            continue;
        }

        // Allouer une frame physique
        match frame_allocator::alloc_frame() {
            Some(phys_addr) => {
                // Créer l'entrée PD : Present + Write + Huge
                let pd_entry = phys_addr | 0x83; // P + W + PS

                // Écrire dans la table de pages
                unsafe {
                    core::ptr::write_volatile(pd_entry_addr as *mut u64, pd_entry);
                    // Invalider le TLB
                    core::arch::asm!(
                        "invlpg [{addr}]",
                        addr = in(reg) HEAP_START + (i as u64 * 2 * 1024 * 1024),
                    );
                }

                serial::print("  Mappe PD[");
                print_num(pd_index);
                serial::print("] virt=");
                print_hex(HEAP_START + (i as u64 * 2 * 1024 * 1024));
                serial::print(" -> phys=");
                print_hex(phys_addr);
                serial::println("");
                mapped += 1;
            }
            None => {
                serial::println("  ERREUR : plus de frames disponibles !");
                return false;
            }
        }
    }

    serial::print("  ");
    print_num(mapped);
    serial::println(" frames mappees pour le heap");
    serial::println("=== Heap mappe ===\n");
    true
}

/// Teste le heap en écrivant et lisant des données
pub fn test_heap() {
    serial::println("=== Test du heap ===");

    // Test 1 : écriture simple
    serial::print("  Test ecriture ");
    print_hex(HEAP_START);
    serial::print(" : ");
    unsafe {
        let ptr = HEAP_START as *mut u64;
        core::ptr::write_volatile(ptr, 0xDEADBEEF_CAFEBABE);
        let val = core::ptr::read_volatile(ptr);
        if val == 0xDEADBEEF_CAFEBABE {
            serial::println("OK");
        } else {
            serial::println("ERREUR !");
        }
    }

    // Test 2 : écriture en fin de heap
    let end_addr = HEAP_END - 8;
    serial::print("  Test ecriture ");
    print_hex(end_addr);
    serial::print(" : ");
    unsafe {
        let ptr = end_addr as *mut u64;
        core::ptr::write_volatile(ptr, 0x1234_5678_9ABC_DEF0);
        let val = core::ptr::read_volatile(ptr);
        if val == 0x1234_5678_9ABC_DEF0 {
            serial::println("OK");
        } else {
            serial::println("ERREUR !");
        }
    }

    // Test 3 : remplir une partie du heap
    serial::print("  Remplissage 1Ko depuis ");
    print_hex(HEAP_START);
    serial::print(" : ");
    unsafe {
        let base = HEAP_START as *mut u64;
        for i in 0..128usize { // 128 × 8 = 1024 octets
            core::ptr::write_volatile(base.add(i), i as u64);
        }
        let mut ok = true;
        for i in 0..128usize {
            if core::ptr::read_volatile(base.add(i)) != i as u64 {
                ok = false;
                break;
            }
        }
        if ok { serial::println("OK"); } else { serial::println("ERREUR !"); }
    }

    serial::println("=== Fin test heap ===\n");
}

// ─── Utilitaires ─────────────────────────────────────────────────────────────

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
