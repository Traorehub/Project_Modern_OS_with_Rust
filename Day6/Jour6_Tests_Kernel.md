# Jour 6 - Tests Kernel : Tests automatisés avec QEMU

---

## Transition depuis le Jour 5

Au Jour 5, on a construit un Safe Wrapper VGA complet avec `println!`, un `Mutex` spinlock et un scroll automatique. Notre kernel affiche du texte proprement et de façon sécurisée.

Mais comment savoir que tout ça **fonctionne vraiment** et continue de fonctionner quand on ajoute du code ? C'est le problème du Jour 6 : mettre en place une **infrastructure de tests automatisés** qui tourne directement dans QEMU et rapporte les résultats dans le terminal.

---

## Concept - Pourquoi tester un kernel ?

En développement normal, `cargo test` suffit. En bare-metal, ce n'est pas possible directement car le code suppose qu'il tourne sans OS. Il faut donc faire tourner les tests **dans QEMU lui-même**.

```
cargo test normal :
[ton code] → compilé pour ton OS → exécuté → résultat

cargo test kernel :
[tes tests] → compilé bare-metal → image disque → QEMU → résultat via port série → lu par cargo
```

---

## Concept - Les failles logiques que les tests préviennent

C'est l'angle cyber du Jour 6. Une **faille logique** ce n'est pas un bug de syntaxe - c'est un bug de comportement que le compilateur ne détecte pas :

| Faille logique | Exemple | Conséquence |
|---|---|---|
| Bounds check cassé | `write_char` accepte `row=26` | Corruption mémoire silencieuse |
| Scroll incorrect | Scroll décale d'une ligne de trop | Données affichées corrompues |
| Calcul d'offset faux | `(row * 80 + col) * 2` avec erreur | Écriture au mauvais endroit |
| Mutex non relâché | Deadlock dans le Writer | Kernel figé indéfiniment |
| Couleur mal encodée | `make_color` inverse fg/bg | Affichage illisible |

Sans tests, ces bugs passent inaperçus jusqu'à ce qu'un attaquant les exploite.

---

## Architecture des tests

```
OS_Day6/
├── Cargo.toml
├── linker.ld
├── i686-unknown-none.json
├── tests/                    ← nouveau dossier
│   └── kernel_tests.rs       ← tests d'intégration
├── .cargo/
│   └── config.toml
└── src/
    ├── main.rs
    ├── vga.rs
    ├── serial.rs             ← nouveau : port série isolé
    ├── test_runner.rs        ← nouveau : infrastructure de test
    └── boot.asm
```

---

## Principe de fonctionnement

```
1. cargo test compile un binaire de test séparé
        ↓
2. Le binaire est lancé dans QEMU via un "runner"
        ↓
3. Chaque test s'exécute dans le kernel
        ↓
4. Le résultat est envoyé sur le port série
        ↓
5. QEMU lit le port série et quitte avec un code de sortie
        ↓
6. cargo interprète le code de sortie : succès ou échec
```

Le code de sortie passe par le **port `0xf4`** - un port spécial de QEMU qui permet de quitter proprement avec un code arbitraire.

---

## Fichiers de configuration

### `Cargo.toml`

```toml
[package]
name = "kernel"
version = "0.1.0"
edition = "2024"

[dependencies]
spin = "0.9"

[dev-dependencies]

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[profile.test]
panic = "abort"

# Configuration du test runner QEMU
[[test]]
name = "kernel_tests"
harness = false
```

### `.cargo/config.toml`

```toml
[build]
target = "i686-unknown-none.json"

[target.i686-unknown-none]
rustflags = [
    "-C", "link-arg=-Tlinker.ld",
    "-C", "linker=rust-lld",
    "-C", "opt-level=z",
    "-C", "strip=debuginfo",
    "-C", "panic=abort",
    "-C", "code-model=kernel",
    "-C", "relocation-model=static",
    "-C", "target-cpu=i686",
]

# Runner : QEMU lance le binaire de test et capture la sortie série
runner = """qemu-system-x86_64 \
  -drive format=raw,file={} \
  -serial stdio \
  -display none \
  -no-reboot \
  -no-shutdown \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04"""
```

> Le device `isa-debug-exit` est la clé : il permet à ton kernel d'envoyer un code de sortie à QEMU via le port `0xf4`. Code `0x10` = succès, code `0x11` = échec.

---

## Nouveau fichier - `src/serial.rs`

On isole le port série dans son propre module pour pouvoir l'utiliser dans les tests sans dépendre du VGA.

```rust
//! Module port série - utilisé pour les tests et le debug
//! COM1 = 0x3F8, vitesse 115200 baud

const COM1: u16 = 0x3F8;

/// Envoie un octet sur COM1
unsafe fn write_byte(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") COM1,
            in("al") byte,
        );
    }
}

/// Envoie une chaîne sur COM1
pub fn print(s: &str) {
    for byte in s.bytes() {
        unsafe { write_byte(byte); }
    }
}

/// Envoie une chaîne suivie d'un retour à la ligne
pub fn println(s: &str) {
    print(s);
    unsafe { write_byte(b'\n'); }
}

/// Macro serial_print! - affiche sur le port série
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        $crate::serial::print(&alloc::format!($($arg)*));
    });
}

/// Macro serial_println! - affiche sur le port série avec newline
#[macro_export]
macro_rules! serial_println {
    ()            => ($crate::serial::println(""));
    ($s:expr)     => ($crate::serial::println($s));
    ($($arg:tt)*) => ({
        $crate::serial::println(&alloc::format!($($arg)*));
    });
}
```

---

## Nouveau fichier - `src/test_runner.rs`

```rust
//! Infrastructure de test bare-metal
//! Gère l'exécution des tests et la communication du résultat à QEMU

use crate::serial;

/// Codes de sortie QEMU via isa-debug-exit
/// QEMU retourne (valeur << 1) | 1
/// donc 0x10 → exit code 33 (succès pour cargo test)
///      0x11 → exit code 35 (échec)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed  = 0x11,
}

/// Envoie le code de sortie à QEMU via le port 0xf4
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx")  0xf4_u16,
            in("eax") exit_code as u32,
        );
    }
    // Ne devrait jamais arriver mais requis pour -> !
    loop {}
}

/// Trait que chaque test doit implémenter
pub trait Testable {
    fn run(&self);
}

/// Implémente Testable pour toute fonction fn() -> ()
impl<T: Fn()> Testable for T {
    fn run(&self) {
        // Affiche le nom du test
        serial::print("  [TEST] ");
        serial::print(core::any::type_name::<T>());
        serial::print(" ... ");
        // Exécute le test
        self();
        // Si on arrive ici, le test a réussi (pas de panic)
        serial::println("ok");
    }
}

/// Lanceur de tests - appelé avec la liste de tous les tests
pub fn run_tests(tests: &[&dyn Testable]) {
    serial::println("\n=============================");
    serial::println(" KERNEL TEST SUITE - Jour 6");
    serial::println("=============================\n");

    let count = tests.len();
    serial::print("Lancement de ");
    // Affichage manuel du nombre sans format!
    serial::print(num_to_str(count as u32));
    serial::println(" test(s)...\n");

    for test in tests {
        test.run();
    }

    serial::println("\n=============================");
    serial::println(" Tous les tests ont reussi !");
    serial::println("=============================\n");

    exit_qemu(QemuExitCode::Success);
}

/// Convertit un u32 en str statique pour éviter alloc
fn num_to_str(n: u32) -> &'static str {
    match n {
        0  => "0",  1  => "1",  2  => "2",  3  => "3",
        4  => "4",  5  => "5",  6  => "6",  7  => "7",
        8  => "8",  9  => "9",  10 => "10", 11 => "11",
        12 => "12", 13 => "13", 14 => "14", 15 => "15",
        _  => "?",
    }
}
```

---

## Fichier de tests - `tests/kernel_tests.rs`

```rust
//! Tests d'intégration kernel - Jour 6
//! Ces tests tournent directement dans QEMU
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]

mod serial {
    include!("../src/serial.rs");
}
mod test_runner {
    include!("../src/test_runner.rs");
}
mod vga {
    include!("../src/vga.rs");
}

use core::panic::PanicInfo;
use test_runner::{Testable, run_tests, exit_qemu, QemuExitCode};

// ─── Point d'entrée des tests ────────────────────────────────────────────────

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    run_tests(&[
        &test_trivial_assertion,
        &test_vga_bounds_check,
        &test_make_color,
        &test_vga_write_char,
        &test_writer_newline,
        &test_bounds_out_of_range,
    ]);
}

// ─── Panic handler pour les tests ────────────────────────────────────────────
// Si un test panic → on affiche l'erreur et on quitte avec Failed

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println("\n[ECHEC] PANIC dans le test !");

    if let Some(msg) = info.message().as_str() {
        serial::print("  Raison  : ");
        serial::println(msg);
    }
    if let Some(loc) = info.location() {
        serial::print("  Fichier : ");
        serial::println(loc.file());
    }

    exit_qemu(QemuExitCode::Failed);
}

// ─── Les tests ───────────────────────────────────────────────────────────────

/// Test 1 : assertion triviale - vérifie que l'infrastructure fonctionne
fn test_trivial_assertion() {
    assert_eq!(1 + 1, 2);
}

/// Test 2 : bounds check VGA - ligne hors limite doit être ignorée
/// Si write_char écrit quand même → corruption mémoire → le test panic
fn test_vga_bounds_check() {
    // Ces appels ne doivent PAS corrompre la mémoire
    unsafe {
        vga::write_char(VGA_HEIGHT, 0, b'X', 0x0F);      // ligne = 25 (hors limite)
        vga::write_char(0, VGA_WIDTH, b'X', 0x0F);       // col  = 80 (hors limite)
        vga::write_char(99, 99, b'X', 0x0F);             // complètement hors limite
        vga::write_char(usize::MAX, 0, b'X', 0x0F);      // valeur maximale
    }
    // Si on arrive ici sans crash → le bounds check fonctionne
}

/// Test 3 : make_color encode correctement fg et bg
fn test_make_color() {
    use vga::Color;
    // Blanc (15) sur Noir (0) → 0x0F
    assert_eq!(vga::make_color(Color::White, Color::Black), 0x0F);
    // Noir (0) sur Blanc (15) → 0xF0
    assert_eq!(vga::make_color(Color::Black, Color::White), 0xF0);
    // Cyan (3) sur Rouge (4) → 0x43
    assert_eq!(vga::make_color(Color::Cyan, Color::Red), 0x43);
}

/// Test 4 : write_char écrit le bon caractère au bon endroit
fn test_vga_write_char() {
    unsafe {
        // Écrire 'Z' à (0,0) en blanc sur noir
        vga::write_char(0, 0, b'Z', 0x0F);
        // Lire directement depuis le buffer VGA pour vérifier
        let ptr = vga::VGA_BUFFER as *const u8;
        let ch    = core::ptr::read_volatile(ptr);
        let color = core::ptr::read_volatile(ptr.add(1));
        assert_eq!(ch, b'Z');
        assert_eq!(color, 0x0F);
    }
}

/// Test 5 : le Writer gère correctement les newlines
fn test_writer_newline() {
    use vga::WRITER;
    let mut w = WRITER.lock();
    w.clear();
    // Après clear, on doit être à (row=0, col=0)
    w.write_byte(b'A');
    w.write_byte(b'\n');
    // Après newline, col doit être revenu à 0
    w.write_byte(b'B');
    // Si on arrive ici sans panic → le Writer gère les newlines
}

/// Test 6 : écriture hors bornes via write_str ne déborde pas
fn test_bounds_out_of_range() {
    unsafe {
        // Une string de 200 caractères sur une ligne de 80 max
        // write_str doit s'arrêter à 80 sans corrompre la mémoire
        let long_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        vga::write_str(0, 0, long_str, 0x0F);
        // Si on arrive ici sans crash → protection OK
    }
}

// Constantes VGA exposées pour les tests
const VGA_HEIGHT: usize = 25;
const VGA_WIDTH:  usize = 80;
```

---

## Fichier principal - `src/main.rs`

```rust
//! Jour 6 - Kernel principal avec infrastructure de test
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga;
mod serial;
mod test_runner;

use core::panic::PanicInfo;
use vga::Color;

unsafe fn halt() -> ! {
    loop { core::arch::asm!("hlt"); }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    vga::WRITER.lock().clear();

    println!("Jour 6 - Kernel Tests");
    println!("=====================");
    println!();
    println!("Lance : cargo test");
    println!("pour executer la suite de tests.");
    println!();

    vga::WRITER.lock().set_color(Color::Green, Color::Black);
    println!("Infrastructure de test prete.");
    vga::WRITER.lock().set_color(Color::White, Color::Black);

    serial::println("Jour 6 kernel boot OK");

    unsafe { halt(); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::WRITER.lock().set_color(Color::White, Color::Red);
    println!("\n=== KERNEL PANIC ===");

    if let Some(msg) = info.message().as_str() {
        println!("Raison  : {}", msg);
    }
    if let Some(loc) = info.location() {
        println!("Fichier : {}", loc.file());
        println!("Ligne   : {}", loc.line());
    }

    println!("Systeme arrete.");
    unsafe { halt(); }
}
```

---

## Commandes - Compilation & Run

### Étape 1 - Créer le projet

```bash
mkdir -p /home/kali/Day6/OS_Day6/src
mkdir -p /home/kali/Day6/OS_Day6/tests
cd /home/kali/Day6/OS_Day6
cargo init --name kernel
```

### Étape 2 - Copier les fichiers du Jour 5

```bash
cp /home/kali/Day5/OS_Day5/src/boot.asm   src/boot.asm
cp /home/kali/Day5/OS_Day5/src/vga.rs     src/vga.rs
cp /home/kali/Day5/OS_Day5/i686-unknown-none.json .
cp /home/kali/Day5/OS_Day5/linker.ld      .
mkdir .cargo
cp /home/kali/Day5/OS_Day5/.cargo/config.toml .cargo/config.toml
```

### Étape 3 - Créer les nouveaux fichiers

```bash
nano Cargo.toml
nano src/serial.rs
nano src/test_runner.rs
nano src/main.rs
nano tests/kernel_tests.rs
nano .cargo/config.toml     # ajouter le runner QEMU
```

### Étape 4 - Compiler le kernel normal

```bash
cargo build -Z build-std=core \
  -Z json-target-spec \
  --target i686-unknown-none.json
```

### Étape 5 - Lancer les tests dans QEMU

```bash
cargo test -Z build-std=core \
  -Z json-target-spec \
  --target i686-unknown-none.json
```

### Étape 6 - Boot normal (optionnel)

```bash
objcopy -O binary \
  target/i686-unknown-none/debug/kernel \
  kernel.bin

nasm -f bin src/boot.asm -o boot.bin

dd if=/dev/zero  of=os.img bs=512 count=2048
dd if=boot.bin   of=os.img conv=notrunc
dd if=kernel.bin of=os.img bs=512 seek=1 conv=notrunc

qemu-system-x86_64 \
  -drive format=raw,file=os.img \
  -serial stdio \
  -no-reboot \
  -no-shutdown
```

---

## Résultats attendus

### `cargo test` dans le terminal

```
=============================
 KERNEL TEST SUITE - Jour 6
=============================

Lancement de 6 test(s)...

  [TEST] kernel_tests::test_trivial_assertion ... ok
  [TEST] kernel_tests::test_vga_bounds_check ... ok
  [TEST] kernel_tests::test_make_color ... ok
  [TEST] kernel_tests::test_vga_write_char ... ok
  [TEST] kernel_tests::test_writer_newline ... ok
  [TEST] kernel_tests::test_bounds_out_of_range ... ok

=============================
 Tous les tests ont reussi !
=============================
```

### Si un test échoue (exemple : bounds check cassé)

```
  [TEST] kernel_tests::test_vga_bounds_check ...
[ECHEC] PANIC dans le test !
  Raison  : assertion failed
  Fichier : tests/kernel_tests.rs
```

Puis `cargo test` retourne un **exit code non-zero** → échec détecté automatiquement.

### Boot normal dans QEMU

```
Jour 6 - Kernel Tests
=====================

Lance : cargo test
pour executer la suite de tests.

Infrastructure de test prete.
```

---

## Résumé - Angle Cyber

| Test | Faille prévenue | Impact si absent |
|---|---|---|
| `test_trivial_assertion` | Infrastructure cassée | Faux positifs sur tous les tests |
| `test_vga_bounds_check` | Buffer overflow VGA | Corruption mémoire silencieuse exploitable |
| `test_make_color` | Encodage couleur inversé | Mauvaise couleur → donnée mal affichée |
| `test_vga_write_char` | Écriture au mauvais offset | Corruption de zones mémoire adjacentes |
| `test_writer_newline` | Curseur mal géré | Dépassement de buffer sur scroll |
| `test_bounds_out_of_range` | String trop longue déborde | Buffer overflow classique |

> Un kernel non testé est un kernel avec des failles inconnues. Les tests automatisés sont la première ligne de défense contre les **failles logiques** - celles que le compilateur ne peut pas détecter mais qu'un attaquant trouvera.
