#!/bin/bash
# Jour 17 : assemblage boot LBA + helpers, puis kernel Rust.
set -e
cd "$(dirname "$0")"

# Tampon kernel a 0x10000, tables de pages a 0x70000 : 384 Ko max.
MAX_KERNEL_SECTORS=768

echo "[1/5] Assemblage du bootloader..."
nasm -f bin   src/boot.asm            -o boot.bin
nasm -f bin   src/stage2.asm          -o stage2.bin

echo "[2/5] Assemblage des helpers 64 bits..."
nasm -f elf64 src/trigger_df.asm      -o trigger_df.o
nasm -f elf64 src/reload_cs.asm       -o reload_cs.o
nasm -f elf64 src/switch_context.asm  -o switch_context.o

echo "[3/5] Compilation du kernel Rust..."
cargo build

echo "[4/5] Taille reelle et patch du MBR..."
KERNEL=target/x86_64-unknown-none/debug/kernel
objcopy -O binary "$KERNEL" /tmp/day17_size_check.bin

BYTES=$(stat -c%s /tmp/day17_size_check.bin)
SECTORS=$(( (BYTES + 511) / 512 ))
rm -f /tmp/day17_size_check.bin

echo "      Image kernel : $BYTES octets = $SECTORS secteurs (max $MAX_KERNEL_SECTORS)"

if [ "$SECTORS" -gt "$MAX_KERNEL_SECTORS" ]; then
  echo
  echo "ERREUR : le kernel depasse le tampon 0x10000-0x6ffff (384 Ko)."
  echo "  Au-dela, le chargement ecraserait les tables de pages a 0x70000."
  exit 1
fi

chmod +x patch_boot_size.sh
./patch_boot_size.sh boot.bin "$BYTES" "$SECTORS"

echo "[5/5] Image disque bootable (raw MBR, pas un ISO)..."
objcopy -O binary "$KERNEL" kernel.bin
dd if=/dev/zero of=os.img bs=512 count=8192 2>/dev/null
dd if=boot.bin   of=os.img conv=notrunc 2>/dev/null
dd if=stage2.bin of=os.img bs=512 seek=1 conv=notrunc 2>/dev/null
dd if=kernel.bin of=os.img bs=512 seek=2 conv=notrunc 2>/dev/null
cp -f os.img ../os_day17.img

echo
echo "OK : MBR patche, $SECTORS secteurs LBA a charger (plafond 768)."
echo "     Image visiteurs : ../os_day17.img"
echo
echo "Lancer avec :  cargo run        (sortie serie, ideal en SSH)"
echo "          ou :  ./run_demo.sh    (fenetre QEMU graphique)"
echo "          ou :  qemu-system-x86_64 -drive format=raw,file=../os_day17.img -serial stdio"
