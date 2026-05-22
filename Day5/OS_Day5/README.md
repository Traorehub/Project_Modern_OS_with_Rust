# Day 5 – VGA Buffer II : Safe Wrapper & macro println!

## Objectif

Au Jour 4, nous avions un accès direct et `unsafe` au buffer texte VGA. Au Jour 5, nous encapsulons cette logique dans une interface sécurisée (Safe Wrapper) avec un `Mutex` pour gérer les accès concurrents, et nous implémentons le support de la macro `println!` standard de Rust.

## Structure du répertoire

```
OS_Day5/
├─ Cargo.toml               # Manifest Rust avec dépendance 'spin' (Mutex sans OS)
├─ linker.ld                # Script de liaison
├─ i686-unknown-none.json   # Cible 32 bits bare-metal
├─ .cargo/config.toml       # Configuration de compilation
└─ src/
   ├─ boot.asm              # Bootloader éducatif (inchangé)
   ├─ main.rs               # Utilisation de println! (plus aucun unsafe visible)
   └─ vga.rs                # Implémentation du Writer, Scroll, et des macros print!
```

## Compilation & Exécution (sur Kali Linux)

```bash
# Compiler le kernel
cargo build -Z build-std=core -Z json-target-spec --target i686-unknown-none.json

# Convertir en binaire plat
objcopy -O binary target/i686-unknown-none/debug/kernel kernel.bin

# Compiler le bootloader
nasm -f bin src/boot.asm -o boot.bin

# Créer l'image disque
dd if=/dev/zero   of=os.img bs=512 count=2048
dd if=boot.bin    of=os.img conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=1 conv=notrunc

# Lancer QEMU (avec affichage graphique)
qemu-system-x86_64 -drive format=raw,file=os.img -serial stdio -no-reboot -no-shutdown
```

## Résultat attendu

La fenêtre QEMU affichera un texte structuré, avec plusieurs couleurs, un formatage de variables (ex: `x + y = 1379`), et la démonstration du scroll automatique après 25 lignes. Le port série confirmera la bonne exécution avec le message `Jour 5 OK - Safe VGA wrapper operationnel`.

## Aspect Cyber-Sécurité

La création d'un "Safe Wrapper" est un concept fondamental en sécurité système. Les opérations critiques (accès mémoire brut à `0xb8000`) sont isolées dans une seule fonction (`write_cell`). Le reste du système interagit avec une interface sécurisée et contrôlée. De plus, le `Mutex` (spinlock) garantit l'absence de "data races" lors des accès concurrents à l'affichage.
