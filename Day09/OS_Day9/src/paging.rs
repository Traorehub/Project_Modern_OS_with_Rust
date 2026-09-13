//! Jour 9 — Lecture et affichage des tables de pages x86_64

use crate::serial;

/// Lit CR3 — pointe vers la PML4 (niveau 4 des tables de pages)
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
    }
    cr3
}

/// Lit une entrée de table de pages à une adresse physique donnée
/// En bare-metal sans virtual memory manager, adresse physique = virtuelle
unsafe fn read_entry(phys_addr: u64) -> u64 {
    let ptr = phys_addr as *const u64;
    core::ptr::read_volatile(ptr)
}

/// Vérifie si une entrée est présente (bit 0)
fn is_present(entry: u64) -> bool {
    entry & 1 != 0
}

/// Vérifie si c'est une huge page (bit 7)
fn is_huge(entry: u64) -> bool {
    entry & (1 << 7) != 0
}

/// Extrait l'adresse physique d'une entrée (bits 12-51)
fn phys_addr(entry: u64) -> u64 {
    entry & 0x000FFFFF_FFFFF000
}

/// Affiche un u64 en hex sur le port série
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

/// Parcourt et affiche les tables de pages actives
pub fn print_page_tables() {
    serial::println("\n=== Tables de pages actives ===");

    let cr3 = read_cr3();
    serial::print("CR3 (PML4) = ");
    print_hex(cr3 & !0xFFF); // masquer les flags
    serial::println("");

    let pml4_addr = cr3 & !0xFFF;

    // Parcourir les 512 entrées PML4
    for pml4_i in 0..512usize {
        let pml4_entry = unsafe { read_entry(pml4_addr + (pml4_i * 8) as u64) };
        if !is_present(pml4_entry) { continue; }

        serial::print("  PML4[");
        print_hex(pml4_i as u64);
        serial::print("] = ");
        print_hex(pml4_entry);
        serial::println("");

        let pdpt_addr = phys_addr(pml4_entry);

        // Parcourir les entrées PDPT
        for pdpt_i in 0..512usize {
            let pdpt_entry = unsafe { read_entry(pdpt_addr + (pdpt_i * 8) as u64) };
            if !is_present(pdpt_entry) { continue; }

            serial::print("    PDPT[");
            print_hex(pdpt_i as u64);
            serial::print("] = ");
            print_hex(pdpt_entry);
            if is_huge(pdpt_entry) {
                serial::print(" [1GB HUGE PAGE]");
            }
            serial::println("");

            if is_huge(pdpt_entry) { continue; }

            let pd_addr = phys_addr(pdpt_entry);

            // Parcourir les entrées PD
            for pd_i in 0..512usize {
                let pd_entry = unsafe { read_entry(pd_addr + (pd_i * 8) as u64) };
                if !is_present(pd_entry) { continue; }

                serial::print("      PD[");
                print_hex(pd_i as u64);
                serial::print("] = ");
                print_hex(pd_entry);
                if is_huge(pd_entry) {
                    serial::print(" [2MB HUGE PAGE -> ");
                    print_hex(phys_addr(pd_entry));
                    serial::print("]");
                }
                serial::println("");

                if is_huge(pd_entry) { continue; }

                // Niveau PT — afficher seulement les premières entrées
                let pt_addr = phys_addr(pd_entry);
                let mut pt_count = 0;
                for pt_i in 0..512usize {
                    let pt_entry = unsafe { read_entry(pt_addr + (pt_i * 8) as u64) };
                    if !is_present(pt_entry) { continue; }
                    if pt_count < 3 {
                        serial::print("        PT[");
                        print_hex(pt_i as u64);
                        serial::print("] -> ");
                        print_hex(phys_addr(pt_entry));
                        serial::println("");
                    }
                    pt_count += 1;
                }
                if pt_count > 3 {
                    serial::print("        ... ");
                    print_hex(pt_count as u64);
                    serial::println(" pages au total");
                }
            }
        }
    }
    serial::println("=== Fin des tables de pages ===\n");
}
