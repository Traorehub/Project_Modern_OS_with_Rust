//! Jour 10 — Pagination II : parcours tables + bit NX

use crate::serial;

// ─── Constantes ──────────────────────────────────────────────────────────────

/// Bit NX (No-Execute) — bit 63 d'une entrée de table de pages
const NX_BIT: u64 = 1 << 63;

/// Flag Present
const PRESENT: u64 = 1 << 0;

/// Flag Huge Page
const HUGE: u64 = 1 << 7;

// ─── Fonctions de base ───────────────────────────────────────────────────────

pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) cr3); }
    cr3
}

unsafe fn read_entry(addr: u64) -> u64 {
    core::ptr::read_volatile(addr as *const u64)
}

unsafe fn write_entry(addr: u64, value: u64) {
    core::ptr::write_volatile(addr as *mut u64, value);
}

fn is_present(e: u64) -> bool { e & PRESENT != 0 }
fn is_huge(e: u64)    -> bool { e & HUGE != 0 }
fn is_nx(e: u64)      -> bool { e & NX_BIT != 0 }
fn phys_addr(e: u64)  -> u64  { e & 0x000FFFFF_FFFFF000 }

// ─── Affichage hex ───────────────────────────────────────────────────────────

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

// ─── Activation NX ───────────────────────────────────────────────────────────

/// Active le bit NX sur toutes les pages SAUF celles contenant le code kernel
/// Le kernel (.text) est à 0x200000-0x21ffff
/// On active NX sur toutes les autres huge pages
pub unsafe fn enable_nx_on_data_pages() {
    // D'abord activer NX dans EFER (bit 11)
    // Sans ça, le bit NX dans les entrées est ignoré
    let efer_msr: u32 = 0xC0000080;
    let efer: u64;
    core::arch::asm!(
        "rdmsr",
        in("ecx") efer_msr,
        out("rax") efer,
        out("rdx") _,
    );

    // Vérifier si NX est déjà activé (bit 11 de EFER)
    if efer & (1 << 11) == 0 {
        // Activer NX execute disable
        let new_efer = efer | (1 << 11);
        core::arch::asm!(
            "wrmsr",
            in("ecx") efer_msr,
            in("rax") new_efer,
            in("rdx") (new_efer >> 32),
        );
        serial::println("  EFER.NXE active");
    } else {
        serial::println("  EFER.NXE deja actif");
    }

    let cr3 = read_cr3();
    let pml4_addr = cr3 & !0xFFF;

    for pml4_i in 0..512usize {
        let pml4_entry = read_entry(pml4_addr + (pml4_i * 8) as u64);
        if !is_present(pml4_entry) { continue; }

        let pdpt_addr = phys_addr(pml4_entry);
        for pdpt_i in 0..512usize {
            let pdpt_entry = read_entry(pdpt_addr + (pdpt_i * 8) as u64);
            if !is_present(pdpt_entry) { continue; }
            if is_huge(pdpt_entry) { continue; }

            let pd_addr = phys_addr(pdpt_entry);
            for pd_i in 0..512usize {
                let pd_entry_addr = pd_addr + (pd_i * 8) as u64;
                let pd_entry = read_entry(pd_entry_addr);
                if !is_present(pd_entry) { continue; }
                if !is_huge(pd_entry) { continue; }

                // Adresse physique de cette huge page
                let page_phys = phys_addr(pd_entry);

                // Kernel .text est à 0x200000
                // On NE met PAS NX sur la page qui contient le kernel
                // On met NX sur toutes les autres pages
                if page_phys == 0x200000 {
                    serial::print("  Page 0x200000 (.text kernel) : NX non applique");
                    serial::println("");
                } else {
                    // Appliquer NX
                    let new_entry = pd_entry | NX_BIT;
                    write_entry(pd_entry_addr, new_entry);
                    // Invalider le TLB pour cette adresse
                    core::arch::asm!(
                        "invlpg [{addr}]",
                        addr = in(reg) page_phys,
                    );
                }
            }
        }
    }
}

/// Affiche le statut NX de chaque huge page
pub fn print_nx_status() {
    serial::println("\n=== Statut NX des pages ===");

    let cr3 = read_cr3();
    let pml4_addr = cr3 & !0xFFF;

    for pml4_i in 0..512usize {
        let pml4_entry = unsafe { read_entry(pml4_addr + (pml4_i * 8) as u64) };
        if !is_present(pml4_entry) { continue; }

        let pdpt_addr = phys_addr(pml4_entry);
        for pdpt_i in 0..512usize {
            let pdpt_entry = unsafe { read_entry(pdpt_addr + (pdpt_i * 8) as u64) };
            if !is_present(pdpt_entry) { continue; }
            if is_huge(pdpt_entry) { continue; }

            let pd_addr = phys_addr(pdpt_entry);
            for pd_i in 0..512usize {
                let pd_entry = unsafe { read_entry(pd_addr + (pd_i * 8) as u64) };
                if !is_present(pd_entry) { continue; }
                if !is_huge(pd_entry) { continue; }

                let page_phys = phys_addr(pd_entry);
                serial::print("  Page ");
                print_hex(page_phys);
                serial::print(" -> ");
                print_hex(page_phys + 0x1FFFFF);
                if is_nx(pd_entry) {
                    serial::println("  [NX - non executable]");
                } else {
                    serial::println("  [RX - executable]");
                }
            }
        }
    }
    serial::println("=== Fin statut NX ===\n");
}

/// Affiche les tables de pages (identique au Jour 9)
pub fn print_page_tables() {
    serial::println("\n=== Tables de pages ===");
    let cr3 = read_cr3();
    serial::print("CR3 = ");
    print_hex(cr3 & !0xFFF);
    serial::println("");

    let pml4_addr = cr3 & !0xFFF;
    for pml4_i in 0..512usize {
        let pml4_entry = unsafe { read_entry(pml4_addr + (pml4_i * 8) as u64) };
        if !is_present(pml4_entry) { continue; }

        let pdpt_addr = phys_addr(pml4_entry);
        for pdpt_i in 0..512usize {
            let pdpt_entry = unsafe { read_entry(pdpt_addr + (pdpt_i * 8) as u64) };
            if !is_present(pdpt_entry) { continue; }
            if is_huge(pdpt_entry) { continue; }

            let pd_addr = phys_addr(pdpt_entry);
            for pd_i in 0..512usize {
                let pd_entry = unsafe { read_entry(pd_addr + (pd_i * 8) as u64) };
                if !is_present(pd_entry) { continue; }
                if !is_huge(pd_entry) { continue; }

                serial::print("  PD[");
                print_hex(pd_i as u64);
                serial::print("] -> ");
                print_hex(phys_addr(pd_entry));
                serial::print(" flags=");
                print_hex(pd_entry & 0xFFF);
                serial::println("");
            }
        }
    }
    serial::println("=== Fin tables ===\n");
}
