# Jour 9 : Pagination I (Résumé Technique)

---

## Introduction

Au Jour 6.5 on a créé des tables de pages manuellement dans `stage2.asm` pour activer le Long Mode. Au Jour 9 on les lit et les comprend depuis le kernel Rust - base de tout ce qui suit.

---

## Structure d'une adresse virtuelle x86_64

```
Bits 47-39 → index PML4  (9 bits = 512 entrées)
Bits 38-30 → index PDPT  (9 bits = 512 entrées)
Bits 29-21 → index PD    (9 bits = 512 entrées)
Bits 20-12 → index PT    (9 bits = 512 entrées)
Bits 11-0  → offset page (12 bits = 4Ko)
```

Chaque table = 512 entrées × 8 octets = **4Ko**.

---

## Huge Pages - notre cas

On utilise des **huge pages 2Mo** (bit 7 dans PD) - on saute le niveau PT :

```
CR3 → PML4[0] → PDPT[0] → PD[0..15] → 16 pages 2Mo = 32Mo
                                         (pas de PT)
```

---

## Flags importants

```
Bit 0  (P)  : Present
Bit 1  (W)  : Writable
Bit 2  (U)  : User accessible
Bit 7  (PS) : Huge page
Bit 63 (NX) : No-Execute (Jour 10)
Bits 12-51  : Adresse physique
```

---

## Code crucial — `src/paging.rs`

```rust
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) cr3); }
    cr3
}

// En identité mapping : adresse virtuelle == physique
// → on peut déréférencer directement les pointeurs vers les tables
unsafe fn read_entry(phys_addr: u64) -> u64 {
    core::ptr::read_volatile(phys_addr as *const u64)
}

fn is_present(e: u64) -> bool { e & 1 != 0 }
fn is_huge(e: u64)    -> bool { e & (1 << 7) != 0 }
fn phys_addr(e: u64)  -> u64  { e & 0x000FFFFF_FFFFF000 }
```

**Point clé :** en identité mapping (virtuel = physique), on peut lire les tables directement par déréférencement. Sans ça, il faudrait un physical memory mapper dédié.

---

## Arborescence

![Structure des fichiers et dossiers](./arborescence_day9.png)

```
OS_Day9/src/
├── main.rs      ← "Jour 9 - Pagination I"
├── paging.rs    ← NOUVEAU
└── ...
```

---

## Résultat obtenu

![Rendu sur Kali QEMU](./Rendu_sur_kali_day9.png)

```
CR3 (PML4) = 0x70000
  PML4[0x0] = 0x71023
    PDPT[0x0] = 0x72023
      PD[0x0] = 0xe3      [2MB HUGE PAGE -> 0x0]
      PD[0x1] = 0x2000e3  [2MB HUGE PAGE -> 0x200000]  ← kernel
      PD[0x2] = 0x400083  [2MB HUGE PAGE -> 0x400000]
      ...
      PD[0xf] = 0x1e00083 [2MB HUGE PAGE -> 0x1e00000]
```

16 huge pages × 2Mo = **32Mo mappés en identité**.

---

## Angle Cyber

| Problème actuel | Solution future |
|---|---|
| Toute la RAM accessible R/W/X | Bit NX → Jour 10 |
| Granularité 2Mo (pas 4Ko) | Pages 4Ko → Jour 11 |
| Pas d'isolation processus | Espaces d'adressage séparés → Jour 12+ |
| Kernel mappé sans protection | SMEP/SMAP → après Jour 12 |

> Notre mapping actuel est **identité** (virtuel = physique) et **flat** (tout R/W/X). C'est l'état d'un système avant toute mitigation. Les Jours 10-12 ajoutent progressivement les protections. Comprendre ce point de départ non sécurisé est essentiel pour comprendre pourquoi chaque mitigation existe.
