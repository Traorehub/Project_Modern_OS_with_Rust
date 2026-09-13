# Jour 8 - Double Fault & IST (Interrupt Stack Table)

---

## Introduction

Au Jour 7, on a configuré l'IDT avec 5 handlers d'exceptions. Le Double Fault handler existait mais utilisait la **même stack** que le kernel. Si cette stack est corrompue - lors d'un stack overflow ou d'une corruption mémoire - le handler lui-même plante, ce qui cause un **Triple Fault** : le CPU reboot instantanément sans laisser aucune trace forensique.

Au Jour 8, on donne au Double Fault handler sa **propre stack dédiée** via l'IST. Cette stack est complètement indépendante de la stack normale - le CPU y switche automatiquement avant d'exécuter le handler, garantissant son exécution même quand tout le reste est détruit.

---

## Concepts fondamentaux

### IST - Interrupt Stack Table

L'IST est une fonctionnalité x86_64 qui permet de définir jusqu'à **7 stacks de secours** (IST[0] à IST[6]). Quand une exception se produit avec un IST index configuré dans l'IDT, le CPU switche **automatiquement** vers cette stack avant d'exécuter le handler.

```
Stack normale corrompue :
Double Fault → CPU essaie d'utiliser la stack corrompue
            → GPF dans le handler
            → Triple Fault → reboot silencieux → aucune trace

Avec IST :
Double Fault → CPU lit IST[0] dans la TSS
            → CPU switche vers 0x380000 (stack dédiée)
            → handler s'exécute proprement sur stack propre
            → [DOUBLE FAULT] IST OK! affiché → halt propre
```

### TSS - Task State Segment

La TSS est une structure mémoire que le CPU consulte pour trouver les adresses des stacks IST. Elle contient le tableau `interrupt_stack_table[7]`. Le CPU la localise via le registre **TR** (Task Register) qui pointe vers un descripteur dans la GDT.

### GDT - Global Descriptor Table

La GDT doit contenir un **descripteur TSS** pour que le CPU sache où trouver la TSS. En 64 bits, ce descripteur occupe **16 octets** (2 entrées de 8 octets) car l'adresse de la TSS est sur 64 bits.

### Chaîne complète

```
IDT[8] (Double Fault)
    → IST index = 1 (1-based dans la crate x86_64)
    → CPU lit TR
    → TR → GDT[descripteur TSS]
    → TSS.interrupt_stack_table[0] (0-based)
    → adresse 0x380000
    → CPU place RSP = 0x380000
    → handler s'exécute
```

---

## Difficultés rencontrées et solutions

### 1. Rechargement de CS après `lgdt`

Quand on charge une nouvelle GDT avec `lgdt`, le registre CS pointe encore vers l'ancien descripteur. Il faut le recharger via un **far return** (`retfq`) - la seule instruction qui recharge CS atomiquement en 64 bits.

La syntaxe correcte avec LLVM inline asm :

```rust
let cs_val = cs_sel.0 as u64;
core::arch::asm!(
    "push {sel}",
    "lea {tmp}, [rip + 2f]",
    "push {tmp}",
    "rex64 retf",   // far return 64 bits
    "2:",
    sel = in(reg) cs_val,
    tmp = lateout(reg) _,
);
```

> `retfq` ne fonctionne pas avec LLVM - utiliser `rex64 retf`.
> Les labels `0:` et `1:` sont interdits en inline asm LLVM (ambiguïté avec les littéraux binaires) - utiliser `2:` minimum.

### 2. Index IST 0-based vs 1-based

La crate `x86_64` utilise un indexage **1-based** pour l'IST dans l'IDT - `DOUBLE_FAULT_IST_INDEX = 0` dans l'IDT correspond à **IST[0]** dans la TSS (index 0 dans le tableau Rust). Cela donne `IST = 1` dans l'entrée IDT binaire.

```
IDT entry bits 32-34 : IST = 1
TSS interrupt_stack_table[0] = 0x380000
→ Le CPU cherche IST[1] (1-based) = IST[0] (0-based) ✅
```

### 3. Stack IST dans `.bss` vs `.data`

`objcopy -O binary` n'inclut **pas** la section `.bss` dans le binaire plat - elle est censée être initialisée à zéro par l'OS. En bare-metal, personne ne le fait.

```rust
// DANGEREUX - dans .bss → absent du kernel.bin → adresse invalide
static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

// CORRECT - dans .data → présent dans kernel.bin
static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0xAB; STACK_SIZE];
```

Dans notre implémentation finale, on utilise une **adresse fixe** `0x380000` dans le mapping du stage2 - ce qui évite complètement ce problème.

### 4. Double Fault handler en ASM pur

Le compilateur Rust génère un **prologue** pour chaque fonction - `push rax`, `sub rsp, N` - même pour un handler minimaliste. Ce prologue accède à la stack avant toute instruction métier. Si la stack IST n'est pas correctement configurée, ce prologue cause un Page Fault → Triple Fault.

Solution : écrire le Double Fault handler **entièrement en NASM** pour zéro prologue :

```nasm
double_fault_handler_asm:
    ; Écriture directe sur VGA ligne 24 - zéro dépendance
    mov byte [0xb8F00], '['
    mov byte [0xb8F01], 0x4F   ; blanc sur rouge
    ; ... etc
    mov dx, 0x3F8              ; COM1
    mov al, 'D'
    out dx, al
    ; ...
.halt:
    hlt
    jmp .halt
```

### 5. Warm up du TLB

La zone mémoire `0x380000` peut ne pas être dans le TLB au moment du Double Fault. Si le CPU essaie de switcher vers cette adresse et qu'elle n'est pas dans le TLB, il doit faire un **page walk** - qui peut échouer si les tables de pages ne sont pas accessibles.

Solution : accéder à `0x380000` avant le test pour charger l'entrée dans le TLB :

```rust
unsafe {
    let warmup = 0x37f000 as *mut u64;
    core::ptr::write_volatile(warmup, 0);
    let warmup2 = 0x380000 as *mut u64;
    core::ptr::write_volatile(warmup2, 0);
}
```

---

## Architecture du projet

```
OS_Day8/
├── Cargo.toml              ← x86_64 v0.15 avec abi_x86_interrupt
├── linker.ld               ← ELF64, _start à 0x200000
├── boot.bin                ← boot sector 16 bits
├── stage2.bin              ← transition 32→64 bits (mappe 32Mo)
├── kernel.bin              ← kernel Rust compilé
├── os.img                  ← image disque finale
├── trigger_df.o            ← objet ASM compilé
├── run_tests.sh
├── .cargo/
│   └── config.toml
└── src/
    ├── main.rs             ← init GDT→IDT, warm up TLB, test
    ├── gdt.rs              ← NOUVEAU : GDT + TSS + IST via crate x86_64
    ├── idt.rs              ← Double Fault avec set_stack_index + handler ASM
    ├── trigger_df.asm      ← NOUVEAU : trigger + handler ASM pur
    ├── vga1.rs
    ├── vga2.rs
    ├── vga.rs
    ├── serial.rs
    ├── test_runner.rs
    ├── boot.asm
    └── stage2.asm
```

---

## Fichiers complets

### `src/gdt.rs`

```rust
use core::ptr::addr_of_mut;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const IST_STACK_TOP: u64 = 0x380000;  // adresse fixe dans mapping stage2

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut CS_SEL:  SegmentSelector = SegmentSelector::NULL;
static mut TSS_SEL: SegmentSelector = SegmentSelector::NULL;

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use crate::serial;

    unsafe {
        // 1. Configurer IST[0] dans la TSS
        (*addr_of_mut!(TSS))
            .interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            VirtAddr::new(IST_STACK_TOP);

        // 2. Construire la GDT via la crate x86_64
        let gdt = &mut *addr_of_mut!(GDT);
        let cs_sel  = gdt.append(Descriptor::kernel_code_segment());
        let tss_sel = gdt.append(Descriptor::tss_segment(&*addr_of_mut!(TSS)));
        *addr_of_mut!(CS_SEL)  = cs_sel;
        *addr_of_mut!(TSS_SEL) = tss_sel;

        // 3. Charger la GDT
        gdt.load();

        // 4. Recharger CS via far return (rex64 retf)
        // Obligatoire après lgdt - CS pointe encore sur l'ancienne GDT
        let cs_val = cs_sel.0 as u64;
        core::arch::asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "rex64 retf",           // far return 64 bits
            "2:",
            sel = in(reg) cs_val,
            tmp = lateout(reg) _,
        );

        // 5. Charger la TSS
        load_tss(tss_sel);
    }
}
```

### `src/idt.rs` - Double Fault avec IST

```rust
use core::ptr::addr_of_mut;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::{serial, gdt, vga, vga2};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init() {
    unsafe {
        let idt = &mut *addr_of_mut!(IDT);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        // set_stack_index() configure l'IST dans l'entrée IDT
        // DOUBLE_FAULT_IST_INDEX = 0 → IST field = 1 (1-based)
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

        idt.load();
    }
}

extern "x86-interrupt" fn double_fault_handler(
    _frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    // Délègue immédiatement au handler ASM pur
    // Le prologue Rust minimal appelle juste cette fonction
    unsafe extern "C" { fn double_fault_handler_asm() -> !; }
    unsafe { double_fault_handler_asm() }
}

extern "x86-interrupt" fn page_fault_handler(
    _frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    serial::println("\n[PAGE FAULT] Corruption stack -> Double Fault...");
    unsafe {
        core::arch::asm!(
            "mov rsp, 0x5000000",  // RSP hors mapping → toute exception → Double Fault
            "xor rax, rax",
            "div rax",
            options(nostack, noreturn)
        );
    }
}

// ... autres handlers identiques au Jour 7
```

### `src/trigger_df.asm` - Trigger + Handler ASM pur

```nasm
section .text
global trigger_double_fault
global double_fault_handler_asm

; Déclenche la chaîne : Page Fault → corruption stack → Double Fault
trigger_double_fault:
    mov rax, 0x5000000          ; adresse hors mapping
    mov qword [rax], 0          ; → Page Fault
    ret

; Handler Double Fault pur ASM - zéro prologue, zéro dépendance Rust
; S'exécute sur IST[0] = 0x380000 quelle que soit l'état de la stack normale
double_fault_handler_asm:
    ; Écrire "[DOUBLE FAULT] IST OK!" sur la ligne 24 du VGA
    ; Ligne 24 = offset 0xF00 depuis 0xb8000
    ; Couleur 0x4F = blanc sur rouge
    mov byte [0xb8F00], '['
    mov byte [0xb8F01], 0x4F
    mov byte [0xb8F02], 'D'
    mov byte [0xb8F03], 0x4F
    mov byte [0xb8F04], 'O'
    mov byte [0xb8F05], 0x4F
    mov byte [0xb8F06], 'U'
    mov byte [0xb8F07], 0x4F
    mov byte [0xb8F08], 'B'
    mov byte [0xb8F09], 0x4F
    mov byte [0xb8F0A], 'L'
    mov byte [0xb8F0B], 0x4F
    mov byte [0xb8F0C], 'E'
    mov byte [0xb8F0D], 0x4F
    mov byte [0xb8F0E], ' '
    mov byte [0xb8F0F], 0x4F
    mov byte [0xb8F10], 'F'
    mov byte [0xb8F11], 0x4F
    mov byte [0xb8F12], 'A'
    mov byte [0xb8F13], 0x4F
    mov byte [0xb8F14], 'U'
    mov byte [0xb8F15], 0x4F
    mov byte [0xb8F16], 'L'
    mov byte [0xb8F17], 0x4F
    mov byte [0xb8F18], 'T'
    mov byte [0xb8F19], 0x4F
    mov byte [0xb8F1A], ']'
    mov byte [0xb8F1B], 0x4F
    mov byte [0xb8F1C], ' '
    mov byte [0xb8F1D], 0x4F
    mov byte [0xb8F1E], 'I'
    mov byte [0xb8F1F], 0x4F
    mov byte [0xb8F20], 'S'
    mov byte [0xb8F21], 0x4F
    mov byte [0xb8F22], 'T'
    mov byte [0xb8F23], 0x4F
    mov byte [0xb8F24], ' '
    mov byte [0xb8F25], 0x4F
    mov byte [0xb8F26], 'O'
    mov byte [0xb8F27], 0x4F
    mov byte [0xb8F28], 'K'
    mov byte [0xb8F29], 0x4F
    mov byte [0xb8F2A], '!'
    mov byte [0xb8F2B], 0x4F

    ; Envoyer sur COM1 (port série)
    mov dx, 0x3F8
    mov al, 0x0A
    out dx, al
    mov al, '['
    out dx, al
    mov al, 'D'
    out dx, al
    mov al, 'F'
    out dx, al
    mov al, ' '
    out dx, al
    mov al, 'I'
    out dx, al
    mov al, 'S'
    out dx, al
    mov al, 'T'
    out dx, al
    mov al, ' '
    out dx, al
    mov al, 'O'
    out dx, al
    mov al, 'K'
    out dx, al
    mov al, '!'
    out dx, al
    mov al, 0x0A
    out dx, al

.halt:
    hlt
    jmp .halt
```

### `src/main.rs`

```rust
//! Jour 8 - Double Fault + IST
#![no_std]
#![feature(abi_x86_interrupt)]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;
mod gdt;
mod idt;

use core::panic::PanicInfo;
use core::fmt::Write as _;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

unsafe extern "C" {
    fn trigger_double_fault();
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    serial::println("Jour 8 - Double Fault & IST");
    vga::WRITER.lock().write_fmt(format_args!("Jour 8 - Double Fault & IST\n")).ok();
    serial::println("============================");
    vga::WRITER.lock().write_fmt(format_args!("============================\n\n")).ok();

    // Ordre obligatoire : GDT (TSS + IST) AVANT IDT
    gdt::init();
    serial::println("GDT + TSS charges.");
    vga::WRITER.lock().write_fmt(format_args!("GDT + TSS charges.\n")).ok();

    idt::init();
    serial::println("IDT chargee.");
    vga::WRITER.lock().write_fmt(format_args!("IDT chargee.\n")).ok();

    // Warm up TLB pour la zone IST stack
    unsafe {
        let warmup  = 0x37f000 as *mut u64;
        let warmup2 = 0x380000 as *mut u64;
        core::ptr::write_volatile(warmup,  0);
        core::ptr::write_volatile(warmup2, 0);
    }

    serial::println("\nTest : Double Fault via RSP invalide...");
    vga::WRITER.lock().write_fmt(format_args!("\nTest Double Fault...\n")).ok();

    // Déclenche la chaîne d'exceptions → Double Fault
    unsafe { trigger_double_fault(); }

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n=== KERNEL PANIC ===");
    if let Some(msg) = info.message().as_str() {
        serial::print("Raison  : ");
        serial::println(msg);
    }
    serial::println("Systeme arrete.");
    vga::WRITER.lock().set_color(vga2::Color::White, vga2::Color::Red);
    vga::WRITER.lock().write_fmt(format_args!("\n=== KERNEL PANIC ===\n")).ok();
    if let Some(msg) = info.message().as_str() {
        vga::WRITER.lock().write_fmt(format_args!("Raison : {}\n", msg)).ok();
    }
    vga::WRITER.lock().write_fmt(format_args!("Systeme arrete.\n")).ok();
    unsafe { halt(); }
}
```

---

## Commandes

```bash
# Compiler le handler ASM
nasm -f elf64 src/trigger_df.asm -o trigger_df.o

# Compiler le kernel
cargo build -Z build-std=core --target x86_64-unknown-none

# Créer le binaire plat (.data obligatoire)
objcopy -O binary \
  --only-section=.text \
  --only-section=.rodata \
  --only-section=.data \
  target/x86_64-unknown-none/debug/kernel kernel.bin

# Créer l'image disque
dd if=/dev/zero   of=os.img bs=512 count=8192
dd if=boot.bin    of=os.img conv=notrunc
dd if=stage2.bin  of=os.img bs=512 seek=1 conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=2 conv=notrunc

# Lancer QEMU
qemu-system-x86_64 \
  -drive format=raw,file=os.img \
  -serial stdio \
  -display sdl \
  -no-reboot \
  -no-shutdown
```

---

## Résultat obtenu

### Terminal (port série)

```
Jour 8 - Double Fault & IST
============================
GDT + TSS charges.
IDT chargee.

Test : Double Fault via RSP invalide...
[PAGE FAULT] Corruption stack -> Double Fault...
[DF IST OK!]
← système halt propre
```

### Fenêtre QEMU (VGA)

```
Ligne 0-N  : messages normaux du kernel
...
Ligne 24   : [DOUBLE FAULT] IST OK!   ← blanc sur fond rouge
```

Le message sur la ligne 24 est écrit **directement par le handler ASM** sans passer par le Writer VGA - il s'affiche même si le Mutex, la stack normale et tout le reste sont corrompus.

---

## Résumé - Angle Cyber

| Concept | Sans IST | Avec IST |
|---|---|---|
| Stack overflow | Triple Fault silencieux → reboot | Double Fault attrapé → message + halt |
| Stack corrompue | Handler inutilisable | CPU switche vers 0x380000 automatiquement |
| Trace forensique | Aucune | RIP, message, halt propre |
| Handler ASM pur | N/A | Zéro dépendance → toujours exécutable |
| Warm up TLB | Page fault dans le handler | Zone IST mappée et accessible |

> Un **Double Fault non géré** est une primitive d'attaque classique - l'attaquant provoque un overflow, le système reboot sans log, l'attaque est indétectable. Avec l'IST, chaque Double Fault est capturé et loggé. C'est la base des **kernel crash dump** des OS modernes (Linux `kdump`, Windows Blue Screen).
>
> Le handler ASM pur est aussi une leçon importante : en sécurité système, les handlers d'urgence doivent être **totalement indépendants** du reste du système. C'est exactement ainsi que sont écrits les handlers de sécurité dans les TPM, HSM et firmware UEFI.
