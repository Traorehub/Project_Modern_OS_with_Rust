#!/bin/bash
# Jour 18 : demo graphique, fenetre QEMU (VGA + clavier PS/2)
# et sortie serie dans le terminal.
set -e
cd "$(dirname "$0")"

KERNEL=target/x86_64-unknown-none/debug/kernel
IMG=os.img

if [ ! -f "$KERNEL" ]; then
  echo "Kernel absent : lance d'abord ./build.sh"
  exit 1
fi

objcopy -O binary "$KERNEL" kernel.bin

BYTES=$(stat -c%s kernel.bin)
SECTORS=$(( (BYTES + 511) / 512 ))
./patch_boot_size.sh boot.bin "$BYTES" "$SECTORS"

dd if=/dev/zero  of="$IMG" bs=512 count=8192 2>/dev/null
dd if=boot.bin   of="$IMG" conv=notrunc 2>/dev/null
dd if=stage2.bin of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null
dd if=kernel.bin of="$IMG" bs=512 seek=2 conv=notrunc 2>/dev/null

echo "Image prete : $IMG ($BYTES octets, $SECTORS secteurs)"

if [ -n "$DISPLAY" ] || [ -n "$WAYLAND_DISPLAY" ]; then
  echo
  echo "Fenetre QEMU graphique."
  echo "  - Clique dans la fenetre puis tape au clavier : les touches"
  echo "    partent sur le controleur PS/2 (IRQ1)."
  echo "  - La sortie serie s'affiche ici, dans ce terminal."
  echo "  - Quitter : fermer la fenetre."
  echo

  exec qemu-system-x86_64 \
    -name "Jour 18 - Scheduler preemptif" \
    -drive format=raw,file="$IMG" \
    -serial stdio \
    -no-reboot \
    -no-shutdown
else
  echo
  echo "Aucun serveur graphique detecte : backend curses."
  echo "Sortie serie journalisee dans /tmp/day18_serial.log"
  echo "Quitter : Echap puis 2, taper 'quit'."
  echo

  exec qemu-system-x86_64 \
    -drive format=raw,file="$IMG" \
    -display curses \
    -serial "file:/tmp/day18_serial.log" \
    -no-reboot \
    -no-shutdown
fi
