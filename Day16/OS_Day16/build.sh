#!/bin/bash
# Jour 16 — build complet : assembleur (boot + helpers) puis kernel Rust
set -e
cd "$(dirname "$0")"

# boot.asm charge 60 secteurs a 0x8000 : 1 pour stage2, le reste pour le kernel.
MAX_KERNEL_SECTORS=59

echo "[1/4] Assemblage du bootloader..."
nasm -f bin   src/boot.asm       -o boot.bin
nasm -f bin   src/stage2.asm     -o stage2.bin

echo "[2/4] Assemblage des helpers 64 bits..."
nasm -f elf64 src/trigger_df.asm -o trigger_df.o
nasm -f elf64 src/reload_cs.asm  -o reload_cs.o

echo "[3/4] Compilation du kernel Rust..."
cargo build

echo "[4/4] Verification de la taille embarquable..."
KERNEL=target/x86_64-unknown-none/debug/kernel
objcopy -O binary "$KERNEL" /tmp/day16_size_check.bin

BYTES=$(stat -c%s /tmp/day16_size_check.bin)
SECTORS=$(( (BYTES + 511) / 512 ))
rm -f /tmp/day16_size_check.bin

echo "      Image kernel : $BYTES octets = $SECTORS secteurs (max $MAX_KERNEL_SECTORS)"

if [ "$SECTORS" -gt "$MAX_KERNEL_SECTORS" ]; then
  echo
  echo "ERREUR : le kernel depasse ce que boot.asm charge depuis le disque."
  echo "  La fin du binaire ne sera jamais lue -> sauts vers des zeros"
  echo "  -> 'Opcode invalide' ou 'RIP : 0x3' a l'execution."
  echo
  echo "Pistes, de la moins couteuse a la plus lourde :"
  echo "  1. Eviter '{}' sur des entiers (utiliser serial::print_dec / Writer::write_dec)"
  echo "  2. Compiler en release : cargo run --release"
  echo "  3. Augmenter 'mov al, 60' dans src/boot.asm (64 max en un seul INT 13h)"
  echo "  4. Passer boot.asm en lecture LBA (INT 13h AH=0x42) en plusieurs blocs"
  exit 1
fi

echo
echo "OK — marge restante : $(( MAX_KERNEL_SECTORS - SECTORS )) secteurs"
echo
echo "Lancer avec :  cargo run        (sortie serie, ideal en SSH)"
echo "          ou :  ./run_demo.sh    (ecran VGA en mode texte dans le terminal)"
