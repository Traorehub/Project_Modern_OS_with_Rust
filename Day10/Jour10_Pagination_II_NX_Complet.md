# Jour 10 - Pagination II : Parcours des tables & bit NX

---

## Introduction

Au Jour 9, on a lu les tables de pages et affiché le mapping. Toutes les pages étaient marquées **R/W/X** - lisibles, modifiables et exécutables. C'est dangereux : un attaquant qui peut écrire des données dans une page peut ensuite exécuter ce code.

Au Jour 10, on implémente le **bit NX (No-Execute)** : on marque toutes les pages sauf le code kernel comme non-exécutables. Toute tentative d'exécution d'une page NX déclenche un Page Fault - la protection fondamentale contre les attaques de type **shellcode injection**.

---

## Le bit NX - théorie

### C'est quoi ?

Le bit **NX** (No-Execute), aussi appelé **XD** (Execute Disable) sur Intel ou **EVP** sur AMD, est le **bit 63** d'une entrée de table de pages.

```
Entrée de table de pages 64 bits :
┌──┬─────────────────────────────┬──────────────────┐
│63│        62 - 52              │    51 - 0        │
│NX│       réservés              │  adresse + flags │
└──┴─────────────────────────────┴──────────────────┘
  ↑
  NX = 1 → page non exécutable
  NX = 0 → page exécutable (défaut)
```

### Prérequis : EFER.NXE

Avant d'utiliser le bit NX dans les entrées de tables, il faut l'activer dans le registre **EFER** (Extended Feature Enable Register) — bit 11 appelé **NXE** (No-Execute Enable).

```rust
// Lire EFER (MSR 0xC0000080)
core::arch::asm!("rdmsr", in("ecx") 0xC0000080u32, out("rax") efer, ...);

// Activer NXE (bit 11)
let new_efer = efer | (1 << 11);
core::arch::asm!("wrmsr", in("ecx") 0xC0000080u32, in("rax") new_efer, ...);
```

Sans `EFER.NXE = 1`, le bit 63 est un **reserved bit** — l'écrire cause un GPF.

### `invlpg` — invalider le TLB

Après avoir modifié une entrée de table de pages, le CPU peut encore utiliser l'ancienne entrée depuis son **TLB** (Translation Lookaside Buffer — cache des traductions d'adresses). Il faut invalider l'entrée :

```rust
core::arch::asm!(
    "invlpg [{addr}]",
    addr = in(reg) page_phys_addr,
);
```

Sans `invlpg`, le NX ne prend pas effet immédiatement.

---

## Ce qu'on fait concrètement

```
Avant NX :
PD[0]  → 0x000000  flags=0xe3  [Present+Write+Accessed+Dirty+Huge] → RWX
PD[1]  → 0x200000  flags=0xe3  [Present+Write+Accessed+Dirty+Huge] → RWX (kernel)
PD[2]  → 0x400000  flags=0x83  [Present+Write+Huge]                → RWX
...

Après NX :
PD[0]  → 0x000000  [NX - non executable]
PD[1]  → 0x200000  [RX - executable]     ← seule page exécutable
PD[2]  → 0x400000  [NX - non executable]
...toutes les autres [NX]
```

On préserve l'exécutabilité **uniquement** de la page `0x200000` qui contient le kernel `.text`.

---

## Implémentation — `src/paging.rs`

### Activation de NX

```rust
const NX_BIT: u64 = 1 << 63;

pub unsafe fn enable_nx_on_data_pages() {
    // 1. Activer EFER.NXE
    let efer_msr: u32 = 0xC0000080;
    let efer: u64;
    core::arch::asm!(
        "rdmsr",
        in("ecx") efer_msr,
        out("rax") efer,
        out("rdx") _,
    );

    if efer & (1 << 11) == 0 {
        let new_efer = efer | (1 << 11);
        core::arch::asm!(
            "wrmsr",
            in("ecx") efer_msr,
            in("rax") new_efer,
            in("rdx") (new_efer >> 32),
        );
    }

    // 2. Parcourir les PD entries et appliquer NX
    let cr3 = read_cr3();
    // ... parcours PML4 → PDPT → PD ...
    for pd_i in 0..512usize {
        let pd_entry = read_entry(pd_addr + (pd_i * 8) as u64);
        if !is_present(pd_entry) || !is_huge(pd_entry) { continue; }

        let page_phys = phys_addr(pd_entry);

        if page_phys == 0x200000 {
            // Kernel .text → ne pas mettre NX
            continue;
        }

        // Appliquer NX + invalider TLB
        let new_entry = pd_entry | NX_BIT;
        write_entry(pd_entry_addr, new_entry);
        core::arch::asm!(
            "invlpg [{addr}]",
            addr = in(reg) page_phys,
        );
    }
}
```

### Vérification du statut NX

```rust
pub fn print_nx_status() {
    // Parcourir les tables et afficher pour chaque page :
    // [NX - non executable] ou [RX - executable]
    if is_nx(pd_entry) {
        serial::println("  [NX - non executable]");
    } else {
        serial::println("  [RX - executable]");
    }
}
```

---

## Arborescence

![Structure des fichiers et dossiers](./arborescence_day10.png)

```
OS_Day10/
└── src/
    ├── main.rs      ← "Jour 10 - Pagination II"
    ├── paging.rs    ← mis à jour : enable_nx_on_data_pages + print_nx_status
    ├── gdt.rs
    ├── idt.rs
    └── ...
```

---

## Commandes

```bash
cargo build -Z build-std=core --target x86_64-unknown-none

objcopy -O binary \
  --only-section=.text \
  --only-section=.rodata \
  --only-section=.data \
  target/x86_64-unknown-none/debug/kernel kernel.bin

dd if=/dev/zero   of=os.img bs=512 count=8192
dd if=boot.bin    of=os.img conv=notrunc
dd if=stage2.bin  of=os.img bs=512 seek=1 conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=2 conv=notrunc

qemu-system-x86_64 \
  -drive format=raw,file=os.img \
  -serial stdio \
  -display none \
  -no-reboot -no-shutdown
```

---

## Résultat obtenu

![Rendu - État initial et activation EFER.NXE](./rendu1.png)

![Rendu - Résultat final après application du bit NX](./rendu_2.png)

```
Jour 10 - Pagination II : bit NX
=================================
GDT + IDT initialises.

--- Avant activation NX ---
CR3 = 0x70000
  PD[0x0] -> 0x0       flags=0xe3
  PD[0x1] -> 0x200000  flags=0xe3
  PD[0x2] -> 0x400000  flags=0x83
  ...

Activation du bit NX sur les pages de donnees...
  EFER.NXE active
  Page 0x200000 (.text kernel) : NX non applique

Bit NX active.

--- Apres activation NX ---
  Page 0x0       -> 0x1fffff   [NX - non executable]
  Page 0x200000  -> 0x3fffff   [RX - executable]      ← kernel
  Page 0x400000  -> 0x5fffff   [NX - non executable]
  Page 0x600000  -> 0x7fffff   [NX - non executable]
  ... (toutes les autres NX)

Pagination II OK.
```

---

## Résumé - Angle Cyber

### Le bit NX contre les attaques de type shellcode

```
Sans NX :
Attaquant écrit du shellcode dans une variable (stack/heap/BSS)
    ↓
Attaquant redirige RIP vers le shellcode (buffer overflow)
    ↓
CPU exécute le shellcode -> compromission totale

Avec NX :
Attaquant écrit du shellcode dans une variable
    ↓
Attaquant redirige RIP vers le shellcode
    ↓
CPU -> page NX -> Page Fault (error code bit 4 = 1 = "instruction fetch")
    ↓
Kernel reprend le contrôle -> attaque bloquée
```

### Les limites du NX

| Attaque | NX suffit ? |
|---|---|
| Shellcode injection classique | ✅ Bloquée |
| Return-to-libc (ROP) | ❌ Non - réutilise du code existant |
| JIT spraying | ❌ Non - le code JIT est dans des pages exécutables |
| Data-only attacks | ❌ Non - manipulation de données sans exécution |

> Le NX est nécessaire mais pas suffisant. Il s'associe à **ASLR** (randomisation des adresses) pour rendre les attaques ROP beaucoup plus difficiles, et à **CFI** (Control Flow Integrity) pour les contrer complètement. Notre kernel a maintenant le NX - les Jours 11-12 ajoutent la gestion mémoire qui permettra d'isoler les espaces d'adressage.

### Flags observés et leur signification

```
flags=0xe3 = 0b11100011
  bit 0 (P)  = 1 -> Present
  bit 1 (W)  = 1 -> Writable
  bit 5 (A)  = 1 -> Accessed (CPU l'a mis à 1)
  bit 6 (D)  = 1 -> Dirty (CPU l'a mis à 1)
  bit 7 (PS) = 1 -> Huge Page 2Mo

flags=0x83 = 0b10000011
  bit 0 (P)  = 1 -> Present
  bit 1 (W)  = 1 -> Writable
  bit 7 (PS) = 1 -> Huge Page 2Mo
  (A et D = 0 car pas encore accédés)

Après NX : bit 63 = 1 ajouté sur toutes les pages sauf 0x200000
```
