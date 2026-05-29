# Jour 7 — Exceptions CPU & IDT

---

## Transition depuis le Jour 6.5

Au Jour 6.5, on a switché vers le **Long Mode 64 bits**. Le kernel tourne désormais en x86_64 avec un paging fonctionnel. Mais il manque encore quelque chose de fondamental : quand le CPU rencontre une erreur (division par zéro, accès mémoire invalide, instruction illégale), **personne ne l'attrape**. Sans IDT configurée, n'importe quelle exception cause un Triple Fault — le CPU reboot instantanément.

Au Jour 7, on implémente l'**IDT (Interrupt Descriptor Table)** : la table qui dit au CPU quoi faire pour chaque type d'exception.

---

## Concept — C'est quoi l'IDT ?

L'IDT est une table en mémoire contenant **256 entrées**. Chaque entrée correspond à un type d'interruption ou d'exception et pointe vers un handler :

```
CPU détecte une erreur
        ↓
CPU lit le registre IDTR (pointe vers l'IDT en mémoire)
        ↓
CPU consulte IDT[numéro_exception]
        ↓
IDT[n] contient l'adresse du handler + privilèges
        ↓
CPU sauvegarde l'état (RIP, CS, RFLAGS, RSP, SS)
        ↓
CPU saute vers le handler
        ↓
Handler traite l'erreur → panic, log, ou récupération
```

Sans IDT chargée → **Triple Fault** → CPU reboot → boucle infinie.

---

## Les exceptions CPU implémentées

| Numéro | Nom | Cause | Error Code |
|---|---|---|---|
| 0 | Divide Error | `div 0` ou division overflow | Non |
| 6 | Invalid Opcode | Instruction invalide ou non supportée | Non |
| 8 | Double Fault | Exception pendant le traitement d'une exception | Oui (toujours 0) |
| 13 | General Protection Fault | Violation de segment, accès privilégié | Oui |
| 14 | Page Fault | Accès à une page non mappée ou sans permission | Oui |

---

## Angle Cyber — Pourquoi ces exceptions sont critiques

### Division par zéro (0)
Simple à déclencher, permet de tester que l'IDT fonctionne. En exploitation, peut être utilisée pour DoS si non gérée.

### Invalid Opcode (6)
Utilisé en **anti-debug** — certains malwares insèrent des instructions invalides (`UD2`) pour détecter si un debugger intercepte l'exception avant eux.

### Double Fault (8)
Se produit quand une exception arrive **pendant le traitement d'une autre exception**. Souvent signe d'une **corruption de stack** — très exploitable car le CPU utilise la même stack corrompue pour handler le double fault. En pratique, un double fault non géré → Triple Fault → reboot.

### General Protection Fault (13)
C'est **l'exception de sécurité principale**. Déclenchée par :
- Accès à de la mémoire non autorisée
- Exécution de code en Ring 0 depuis Ring 3
- Utilisation de segments invalides

La majorité des exploits de type **privilege escalation** tentent de déclencher un GPF contrôlé pour rediriger l'exécution.

### Page Fault (14)
Déclenchée quand le CPU accède à une adresse virtuelle non mappée ou sans permission. Le registre **CR2** contient l'adresse qui a causé le fault — information clé pour le debugging et la forensics.

```
Error code du Page Fault :
bit 0 (P)  : 0 = page absente, 1 = violation de protection
bit 1 (W)  : 0 = lecture, 1 = écriture
bit 2 (U)  : 0 = kernel, 1 = userspace
bit 3 (R)  : reserved bit violation
bit 4 (I)  : instruction fetch (NX violation)
```

Un **buffer overflow** qui déborde sur une page non mappée déclenche un Page Fault — c'est le mécanisme de détection de base des OS modernes.

---

## La crate `x86_64`

On utilise la crate `x86_64` qui fournit des abstractions typées pour le CPU :

```
Sans x86_64 :
- Structs IDT manuelles (200+ lignes)
- Calcul manuel des offsets et flags
- Gestion manuelle du registre IDTR

Avec x86_64 :
- InterruptDescriptorTable → table complète typée
- InterruptStackFrame → état CPU sauvegardé par le CPU
- set_handler_fn() → enregistrement typé et vérifié
- idt.load() → instruction LIDT correcte
```

Elle ne gère pas la logique OS — c'est juste une abstraction sur le hardware x86_64.

**Feature obligatoire :** `abi_x86_interrupt` — l'ABI spéciale pour les handlers d'interruption qui préserve tous les registres automatiquement.

---

## Point critique — `addr_of_mut!`

Rust 2024 interdit les références directes à un `static mut`. Pour modifier l'IDT statique sans créer de référence interdite, on utilise `addr_of_mut!` :

```rust
// INTERDIT en Rust 2024 :
IDT.divide_error.set_handler_fn(handler); // référence à static mut

// CORRECT :
let idt = &mut *addr_of_mut!(IDT);        // pointeur brut → référence locale
idt.divide_error.set_handler_fn(handler);
```

C'est une contrainte de sécurité importante — Rust 2024 rend explicite le danger des références globales mutables.

---

## Structure du projet

```
OS_Day7/
├── Cargo.toml          ← dépendance x86_64 avec feature abi_x86_interrupt
├── src/
│   ├── main.rs         ← initialise l'IDT, déclenche les tests
│   ├── idt.rs          ← nouveau : IDT + handlers
│   ├── vga1.rs
│   ├── vga2.rs
│   ├── vga.rs
│   ├── serial.rs
│   ├── test_runner.rs
│   ├── boot.asm
│   └── stage2.asm
└── tests/
    └── src/
        └── main.rs
```

---

## Fichiers

### `Cargo.toml`

```toml
[workspace]
members = [
    ".",
    "tests",
]
resolver = "3"

[package]
name = "kernel"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "kernel"
path = "src/main.rs"
test = false
bench = false

[dependencies]
x86_64 = { version = "0.15", default-features = false, features = ["instructions", "abi_x86_interrupt"] }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[profile.test]
panic = "abort"
```

### `src/idt.rs`

```rust
//! IDT — Interrupt Descriptor Table
//! Jour 7 : gestion des exceptions CPU en 64 bits

use core::ptr::addr_of_mut;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::serial;

/// IDT statique — doit vivre toute la durée du programme
/// Le CPU lit cette table à chaque exception via le registre IDTR
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Initialise l'IDT et la charge dans le CPU via l'instruction LIDT
pub fn init() {
    unsafe {
        // addr_of_mut! évite la référence directe à static mut (interdit Rust 2024)
        let idt = &mut *addr_of_mut!(IDT);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        // Charge l'IDT dans le registre IDTR du CPU
        idt.load();
    }
}

/// Exception 0 — Division par zéro
/// ABI x86-interrupt : le CPU sauvegarde automatiquement RIP, CS, RFLAGS, RSP, SS
extern "x86-interrupt" fn divide_error_handler(frame: InterruptStackFrame) {
    serial::println("\n[EXCEPTION] Division par zero !");
    serial::print("  RIP : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    panic!("divide error");
}

/// Exception 6 — Instruction invalide
/// Utilisé en anti-debug : UD2 déclenche cette exception
extern "x86-interrupt" fn invalid_opcode_handler(frame: InterruptStackFrame) {
    serial::println("\n[EXCEPTION] Opcode invalide !");
    serial::print("  RIP : ");
    print_hex(frame.instruction_pointer.as_u64());
    serial::println("");
    panic!("invalid opcode");
}

/// Exception 8 — Double Fault
/// Se produit quand une exception arrive pendant le traitement d'une exception
/// Retourne ! car on ne peut jamais récupérer d'un double fault
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

/// Exception 13 — General Protection Fault
/// Violation de segment, accès non autorisé, instruction privilégiée
/// L'error code encode le segment selector fautif
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

/// Exception 14 — Page Fault
/// Accès à une page non mappée ou sans permission
/// CR2 contient l'adresse virtuelle fautive — clé pour le debugging
extern "x86-interrupt" fn page_fault_handler(
    frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    // CR2 = adresse virtuelle qui a causé le fault
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
        serial::println("  Cause : violation de protection (page presente)");
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

/// Affiche un u64 en hexadécimal sur le port série
/// Sans allocateur on ne peut pas utiliser format!("{:x}")
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
```

### `src/main.rs`

```rust
//! Jour 7 — IDT et exceptions CPU
#![no_std]
#![feature(abi_x86_interrupt)]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;
mod idt;

use core::panic::PanicInfo;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    serial::println("Jour 7 - IDT & Exceptions CPU");
    serial::println("==============================");

    // Initialiser l'IDT — à faire AVANT tout ce qui pourrait causer une exception
    idt::init();
    serial::println("IDT chargee.");

    // Test : déclencher volontairement une division par zéro
    // Sans IDT → Triple Fault → reboot
    // Avec IDT → handler attrapé → message + panic propre
    serial::println("\nTest 1 : declenchement division par zero...");
    unsafe {
        core::arch::asm!(
            "mov rax, 0",
            "mov rbx, 0",
            "div rbx",       // DIV par zéro → exception 0
        );
    }

    // Ne devrait jamais arriver
    serial::println("Erreur : exception non declenchee !");
    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n=== KERNEL PANIC ===");
    if let Some(msg) = info.message().as_str() {
        serial::print("Raison  : ");
        serial::println(msg);
    }
    if let Some(loc) = info.location() {
        serial::print("Fichier : ");
        serial::println(loc.file());
    }
    serial::println("Systeme arrete.");
    unsafe { halt(); }
}
```

---

## Commandes — Compilation & Run

### Étape 1 — Compiler

```bash
cargo build -Z build-std=core --target x86_64-unknown-none
```

### Étape 2 — Créer l'image et booter

```bash
objcopy -O binary \
  --only-section=.text \
  --only-section=.rodata \
  --only-section=.data \
  --only-section=.bss \
  target/x86_64-unknown-none/debug/kernel kernel.bin

dd if=/dev/zero   of=os.img bs=512 count=8192
dd if=boot.bin    of=os.img conv=notrunc
dd if=stage2.bin  of=os.img bs=512 seek=1 conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=2 conv=notrunc

qemu-system-x86_64 \
  -drive format=raw,file=os.img \
  -serial stdio \
  -display none \
  -no-reboot \
  -no-shutdown
```

---

## Résultat obtenu

```
Jour 7 - IDT & Exceptions CPU
==============================
IDT chargee.

Test 1 : declenchement division par zero...

[EXCEPTION] Division par zero !
  RIP : 0x200050

=== KERNEL PANIC ===
Raison  : divide error
Fichier : src/idt.rs
Systeme arrete.
```

### Interprétation

- **`IDT chargee.`** → l'instruction `LIDT` a bien été exécutée, le registre IDTR pointe vers notre table
- **`[EXCEPTION] Division par zero !`** → le CPU a consulté `IDT[0]` et sauté vers notre handler
- **`RIP : 0x200050`** → adresse exacte de l'instruction `DIV RBX` dans le kernel — permet de localiser précisément l'instruction fautive
- **`Raison : divide error`** → le panic handler a pris le relais proprement
- **Pas de reboot** → sans IDT ça rebootait en boucle, maintenant le CPU halt proprement

---

## Résumé — Angle Cyber

| Exception | Vecteur d'attaque | Ce que notre handler fait |
|---|---|---|
| Divide Error (0) | DoS par crash non géré | Log RIP + panic propre |
| Invalid Opcode (6) | Anti-debug, UD2 traps | Log RIP + panic propre |
| Double Fault (8) | Stack corruption exploit | Log + halt → pas de Triple Fault |
| GPF (13) | Privilege escalation, OOB access | Log RIP + error code + panic |
| Page Fault (14) | Buffer overflow, use-after-free | Log CR2 + type d'accès + panic |

> Le **RIP** affiché par chaque handler est une information forensique clé — il permet de localiser exactement quelle instruction a causé l'exception. En exploitation, un attaquant qui contrôle RIP au moment d'un GPF ou Page Fault peut rediriger l'exécution vers son shellcode. Notre handler le capture avant que ça arrive.
>
> La valeur dans **CR2** lors d'un Page Fault est également critique — si CR2 pointe vers une adresse contrôlée par l'attaquant, c'est souvent le signe d'un **null pointer dereference** ou d'un **use-after-free** exploitable.
