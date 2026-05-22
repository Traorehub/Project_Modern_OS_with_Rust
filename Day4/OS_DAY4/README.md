# Day 4 – VGA Buffer & Kernel

## Objectif

Nous poursuivons le projet OS Rust en ajoutant un **buffer texte VGA** (`0xb8000`). Le kernel bare‑metal écrit directement sur l’écran, montre comment éviter les corruptions mémoire et conserve le même bootloader que le Jour 3.

## Structure du répertoire

```
Day4/OS_DAY4/
├─ Cargo.toml               # Manifest Rust
├─ i686-unknown-none.json  # Cible 32 bits personnalisée
├─ linker.ld                # Script de liaison (entry @0x8000)
├─ .cargo/config.toml      # Configuration de la cible
└─ src/
   ├─ boot.asm             # Bootloader éducatif (identique au Jour 3)
   ├─ main.rs              # Kernel : init VGA, écrit du texte, panic handler
   └─ vga.rs               # Gestion du buffer VGA (couleurs, bounds‑check)
```

## Compilation & exécution

```bash
# Depuis le répertoire Day4/OS_DAY4 (sur votre VM Kali)
cargo build -Z build-std=core --target i686-unknown-none.json
objcopy -O binary target/i686-unknown-none/debug/kernel kernel.bin
nasm -f bin src/boot.asm -o boot.bin
# Crée l’image disque (1 MiB)
 dd if=/dev/zero of=os.img bs=512 count=2048
 dd if=boot.bin of=os.img conv=notrunc
 dd if=kernel.bin of=os.img bs=512 seek=1 conv=notrunc
# Lancer QEMU
default:
 qemu-system-x86_64 -drive format=raw,file=os.img -serial stdio -no-reboot -no-shutdown -display none
```

### Résultat attendu

- **Fenêtre QEMU** : `A` en jaune, puis les messages `Hello from VGA Buffer!` (cyan) et `Out‑of‑bounds write blocked!` (rouge). Le texte `KERNEL PANIC` apparaît en blanc sur fond rouge si un panic survient.
- **Terminal (port série)** : `VGA Buffer test done.`

## Points de sécurité

- Le buffer VGA est un **pointeur brut** (`0xb8000`). Un accès hors limites provoquerait une corruption mémoire sévère – le `write_char` refuse ces écritures.
- Le code montre comment **détecter et bloquer** ce type d’erreur, illustrant la prévention des *buffer‑overflow*.

## Prochaine étape

Intégrer la gestion d’interruptions clavier ou un affichage plus riche (graphique) en continuant la série de Days 4‑5.

---
