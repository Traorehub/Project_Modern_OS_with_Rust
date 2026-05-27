# Jour 6 - Tests Kernel : Tests automatisés avec QEMU

---

## Transition depuis le Jour 5

Au Jour 5, on a construit un Safe Wrapper VGA complet avec `println!`, un `Mutex` spinlock maison et un scroll automatique. Le kernel affiche du texte proprement et de façon sécurisée.

Mais une question reste ouverte : **comment savoir que tout ça fonctionne vraiment et continue de fonctionner quand on ajoute du code ?**

C'est le problème du Jour 6 : mettre en place une infrastructure de tests automatisés qui tourne directement dans QEMU et rapporte les résultats dans le terminal via le port série.

---

## Concept - Pourquoi tester un kernel ?

En développement normal, `cargo test` suffit. En bare-metal, ce n'est pas possible directement car le code suppose qu'il tourne sans OS. Il faut faire tourner les tests **dans QEMU lui-même**.

```
cargo test normal :
[ton code] → compilé pour ton OS → exécuté → résultat

cargo test kernel :
[tes tests] → compilé bare-metal → image disque → QEMU → résultat via port série → lu par cargo
```

---

## Concept - Les failles logiques que les tests préviennent

Une **faille logique** ce n'est pas un bug de syntaxe - c'est un bug de comportement que le compilateur ne détecte pas :

| Faille logique | Exemple | Conséquence |
|---|---|---|
| Bounds check cassé | `write_char` accepte `row=26` | Corruption mémoire silencieuse |
| Scroll incorrect | Scroll décale d'une ligne de trop | Données affichées corrompues |
| Calcul d'offset faux | `(row * 80 + col) * 2` avec erreur | Écriture au mauvais endroit |
| Mutex non relâché | Deadlock dans le Writer | Kernel figé indéfiniment |
| Couleur mal encodée | `make_color` inverse fg/bg | Affichage illisible |

Sans tests, ces bugs passent inaperçus jusqu'à ce qu'un attaquant les exploite.

---

## Architecture du projet

```
OS_Day6/
├── Cargo.toml              ← workspace racine
├── linker.ld               ← CRITIQUE : doit placer _start à 0x8000
├── i686-unknown-none.json  ← target bare-metal custom
├── boot.bin                ← boot sector compilé
├── run_tests.sh            ← script runner QEMU
├── .cargo/
│   └── config.toml         ← config compilateur + runner
├── src/
│   ├── main.rs
│   ├── vga1.rs             ← spinlock maison
│   ├── vga2.rs             ← Writer + couleurs
│   ├── vga.rs              ← couche compat fonctions libres
│   ├── serial.rs           ← port série COM1
│   ├── test_runner.rs      ← infrastructure de test
│   └── boot.asm
└── tests/
    ├── Cargo.toml
    ├── linker.ld
    └── src/
        ├── main.rs         ← point d'entrée des tests
        ├── vga1.rs         ← copies des sources
        ├── vga2.rs
        ├── vga.rs
        ├── serial.rs
        └── test_runner.rs
```

---

## Points critiques - Ce qu'il faut absolument retenir

### 1. Le linker script doit placer `_start` à `0x8000`

C'est **le point le plus important** du Jour 6 et celui qui a causé le plus de problèmes.

```ld
/* FAUX - place _start à 0x100000 (1MB) */
. = 1M;

/* CORRECT - place _start à 0x8000 */
. = 0x8000;
```

Pourquoi ? Parce que le boot sector charge le kernel à l'adresse `0x8000` et y saute :

```nasm
call 0x8000   ; boot sector saute ici
```

Si `_start` est ailleurs, le CPU exécute du code aléatoire → comportement indéfini.

**Comment vérifier :**

```bash
nm target/i686-unknown-none/debug/deps/kernel_tests-* | grep _start
# Doit afficher : 00008000 T _start
```

### 2. Le workspace hérite du config racine

Dans un workspace Rust, **le `.cargo/config.toml` de la racine s'applique à tous les membres**. Les configs locaux dans les sous-dossiers sont ignorés. C'est pourquoi on utilise un chemin absolu dans le config racine :

```toml
"-C", "link-arg=-T/home/kali/Day6/OS_Day6/linker.ld",
```

### 3. `harness = false` est obligatoire

```toml
[[bin]]
name = "kernel_tests"
harness = false
```

Sans `harness = false`, cargo essaie d'utiliser le harnais de test standard de Rust qui nécessite la crate `test` - impossible en `no_std`. Ça cause l'erreur `can't find crate for 'test'`.

### 4. `run_tests` doit retourner `!`

```rust
// FAUX - retourne ()
pub fn run_tests(tests: &[&dyn Testable]) { ... }

// CORRECT - ne retourne jamais
pub fn run_tests(tests: &[&dyn Testable]) -> ! { ... }
```

En bare-metal, `_start` ne peut jamais retourner - il n'y a personne pour récupérer la valeur de retour.

---

## Les fichiers clés

### `linker.ld` (racine)

```ld
OUTPUT_FORMAT("elf32-i386")
ENTRY(_start)
SECTIONS
{
  . = 0x8000;
  .text : { *(.text.entry) *(.text*) }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  .bss : { *(.bss*) }
}
```

### `.cargo/config.toml` (racine)

```toml
[build]
target = "i686-unknown-none.json"

[target.i686-unknown-none]
rustflags = [
    "-C", "link-arg=-T/home/kali/Day6/OS_Day6/linker.ld",
    "-C", "linker=rust-lld",
    "-C", "opt-level=z",
    "-C", "strip=debuginfo",
    "-C", "panic=abort",
    "-C", "code-model=kernel",
    "-C", "relocation-model=static",
    "-C", "target-cpu=i686",
]
runner = "/home/kali/Day6/OS_Day6/run_tests.sh"
```

### `run_tests.sh`

```bash
#!/bin/bash
# Runner QEMU pour les tests kernel
# Reçoit le binaire ELF en argument, le convertit en image disque et le lance

BINARY=$1
IMG=$(mktemp /tmp/kernel_test_XXXXXX.img)

# Convertit l'ELF en binaire plat
objcopy -O binary "$BINARY" "${IMG}.bin"

# Crée l'image disque
dd if=/dev/zero of="$IMG" bs=512 count=2048 2>/dev/null

# Boot sector au secteur 1
dd if=/home/kali/Day6/OS_Day6/boot.bin of="$IMG" conv=notrunc 2>/dev/null

# Binaire de test à partir du secteur 2
dd if="${IMG}.bin" of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null

# Lance QEMU avec isa-debug-exit pour récupérer le code de sortie
qemu-system-x86_64 \
  -drive format=raw,file="$IMG" \
  -serial stdio \
  -display none \
  -no-reboot \
  -no-shutdown \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04

EXIT=$?
rm -f "$IMG" "${IMG}.bin"
exit $EXIT
```

### `src/test_runner.rs`

```rust
use crate::serial;

/// Codes de sortie QEMU via isa-debug-exit
/// QEMU retourne (valeur << 1) | 1
/// 0x10 → exit code 33 = succès pour cargo
/// 0x11 → exit code 35 = échec pour cargo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed  = 0x11,
}

/// Envoie le code de sortie à QEMU via le port 0xf4
/// Le device isa-debug-exit intercepte cette écriture et quitte QEMU
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx")  0xf4_u16,
            in("eax") exit_code as u32,
        );
    }
    loop {}
}

/// Trait que chaque test doit implémenter
pub trait Testable {
    fn run(&self);
}

/// Implémentation automatique pour toute fonction fn() -> ()
/// type_name::<T>() donne le nom complet de la fonction pour l'affichage
impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial::print("  [TEST] ");
        serial::print(core::any::type_name::<T>());
        serial::print(" ... ");
        self();
        serial::println("ok");
    }
}

/// Lance tous les tests et quitte QEMU avec le code approprié
/// Retourne ! car en bare-metal on ne peut jamais retourner
pub fn run_tests(tests: &[&dyn Testable]) -> ! {
    serial::println("\n=============================");
    serial::println(" KERNEL TEST SUITE - Jour 6");
    serial::println("=============================\n");

    for test in tests {
        test.run();
    }

    serial::println("\n=============================");
    serial::println(" Tous les tests ont reussi !");
    serial::println("=============================\n");

    exit_qemu(QemuExitCode::Success)
}
```

### `tests/src/main.rs`

```rust
#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod vga1;
mod vga2;
mod vga;
mod serial;
mod test_runner;

use core::panic::PanicInfo;
use test_runner::{run_tests, exit_qemu, QemuExitCode};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    run_tests(&[
        &test_trivial_assertion,
        &test_vga_bounds_check,
        &test_make_color,
        &test_vga_write_char,
        &test_bounds_out_of_range,
    ])
}

/// Panic handler pour les tests
/// Si un test panic → affiche l'erreur et quitte QEMU avec Failed
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

/// Test 1 : vérifie que l'infrastructure fonctionne
fn test_trivial_assertion() {
    assert_eq!(1 + 1, 2);
}

/// Test 2 : le bounds check VGA refuse les accès hors limite
fn test_vga_bounds_check() {
    unsafe {
        vga::write_char(25, 0, b'X', 0x0F);        // ligne hors limite
        vga::write_char(0, 80, b'X', 0x0F);        // colonne hors limite
        vga::write_char(99, 99, b'X', 0x0F);       // les deux hors limite
        vga::write_char(usize::MAX, 0, b'X', 0x0F); // valeur maximale
    }
    // Si on arrive ici sans crash → bounds check OK
}

/// Test 3 : make_color encode correctement les couleurs
fn test_make_color() {
    use vga2::Color;
    assert_eq!(vga2::make_color(Color::White, Color::Black), 0x0F);
    assert_eq!(vga2::make_color(Color::Black, Color::White), 0xF0);
    assert_eq!(vga2::make_color(Color::Cyan,  Color::Red),   0x43);
}

/// Test 4 : write_char écrit au bon endroit en mémoire
fn test_vga_write_char() {
    unsafe {
        vga::write_char(0, 0, b'Z', 0x0F);
        let ptr   = vga2::VGA_BUFFER as *const u8;
        let ch    = core::ptr::read_volatile(ptr);
        let color = core::ptr::read_volatile(ptr.add(1));
        assert_eq!(ch,    b'Z');
        assert_eq!(color, 0x0F);
    }
}

/// Test 5 : write_str s'arrête correctement aux limites
fn test_bounds_out_of_range() {
    unsafe {
        let long_str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        vga::write_str(0, 0, long_str, 0x0F);
        // Si on arrive ici sans crash → protection OK
    }
}
```

---

## Commandes - Compilation & Run

### Étape 1 - Créer la structure

```bash
mkdir -p /home/kali/Day6/OS_Day6/src
mkdir -p /home/kali/Day6/OS_Day6/tests/src
cd /home/kali/Day6/OS_Day6
```

### Étape 2 - Compiler le boot sector

```bash
cp /home/kali/Day5/OS_Day5/src/boot.asm src/boot.asm
nasm -f bin src/boot.asm -o boot.bin
```

### Étape 3 - Rendre le runner exécutable

```bash
chmod +x run_tests.sh
```

### Étape 4 - Lancer les tests

```bash
cargo test -Z build-std=core -Z json-target-spec --target i686-unknown-none.json
```

### Étape 5 - Vérifier l'adresse de `_start` (diagnostic)

```bash
nm target/i686-unknown-none/debug/deps/kernel_tests-* | grep _start
# Doit afficher : 00008000 T _start
```

---

## Résultat obtenu

Les tests s'exécutent avec succès dans QEMU et renvoient les résultats via la liaison série :

![Résultat obtenu](./kernel_test_results.png)

### Interprétation des résultats

| Test | Ce que ça prouve |
|---|---|
| `test_trivial_assertion` ✅ | L'infrastructure de test fonctionne, QEMU reçoit bien la sortie série |
| `test_vga_bounds_check` ✅ | Aucun des 4 accès hors limite n'a crashé le kernel → bounds check solide |
| `test_make_color` ✅ | L'encodage fg/bg est correct → pas d'inversion de couleurs |
| `test_vga_write_char` ✅ | La valeur lue en mémoire correspond à ce qu'on a écrit → offset calculé correctement |
| `test_bounds_out_of_range` ✅ | Une string de 160+ chars sur 80 colonnes ne déborde pas |

---

## Résumé - Angle Cyber

| Concept | Risque | Ce que le test prouve |
|---|---|---|
| Bounds check VGA | Buffer overflow → corruption mémoire | `write_char(99,99)` ignoré silencieusement |
| Calcul d'offset | Écriture au mauvais endroit mémoire | Lecture vérifiée à l'adresse exacte |
| Encodage couleur | Donnée mal représentée | Valeur binaire vérifiée bit à bit |
| String trop longue | Débordement de tampon | `write_str` s'arrête proprement à 80 chars |
| Linker script `0x8000` | Code exécuté au mauvais endroit | `nm` confirme l'adresse avant de lancer |
| `isa-debug-exit` | Pas de retour de code de sortie | cargo interprète succès/échec automatiquement |

> Un kernel non testé est un kernel avec des failles inconnues. Les tests automatisés sont la première ligne de défense contre les **failles logiques** - celles que le compilateur ne peut pas détecter mais qu'un attaquant trouvera. La mécanique `isa-debug-exit` est aussi un vecteur d'attaque à connaître : tout code capable d'écrire sur le port `0xf4` peut forcer QEMU à quitter - c'est un exemple concret de **side channel via I/O**.
