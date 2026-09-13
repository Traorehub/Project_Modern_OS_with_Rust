#!/bin/bash
# Jour 16 — demo graphique : vraie fenetre QEMU (ecran VGA + clavier PS/2)
# et sortie serie dans le terminal. C'est le mode a utiliser pour les
# captures d'ecran et les enregistrements video.
#
# Si aucun serveur graphique n'est detecte (session SSH sans X11), on
# retombe automatiquement sur le backend curses.
set -e
cd "$(dirname "$0")"

KERNEL=target/x86_64-unknown-none/debug/kernel
IMG=os.img

if [ ! -f "$KERNEL" ]; then
  echo "Kernel absent — lance d'abord ./build.sh"
  exit 1
fi

# ─── Fabrication de l'image disque ──────────────────────────────────────────
objcopy -O binary "$KERNEL" kernel.bin

dd if=/dev/zero  of="$IMG" bs=512 count=8192 2>/dev/null
dd if=boot.bin   of="$IMG" conv=notrunc 2>/dev/null
dd if=stage2.bin of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null
dd if=kernel.bin of="$IMG" bs=512 seek=2 conv=notrunc 2>/dev/null

echo "Image prete : $IMG"

# ─── Lancement ──────────────────────────────────────────────────────────────
if [ -n "$DISPLAY" ] || [ -n "$WAYLAND_DISPLAY" ]; then
  echo
  echo "Fenetre QEMU graphique."
  echo "  - Clique dans la fenetre puis tape au clavier : les touches"
  echo "    partent sur le controleur PS/2 (IRQ1)."
  echo "  - La sortie serie s'affiche ici, dans ce terminal."
  echo "  - Capture propre de l'ecran VGA : Ctrl+Alt+2 puis"
  echo "    'screendump /tmp/jour16.ppm', et Ctrl+Alt+1 pour revenir."
  echo "  - Quitter : fermer la fenetre."
  echo

  exec qemu-system-x86_64 \
    -name "Jour 16 - Timer PIT 8254 (IRQ0)" \
    -drive format=raw,file="$IMG" \
    -serial stdio \
    -no-reboot \
    -no-shutdown
else
  echo
  echo "Aucun serveur graphique detecte -> backend curses."
  echo "Sortie serie journalisee dans /tmp/day16_serial.log"
  echo "Quitter : Echap puis 2, taper 'quit'."
  echo

  exec qemu-system-x86_64 \
    -drive format=raw,file="$IMG" \
    -display curses \
    -serial "file:/tmp/day16_serial.log" \
    -no-reboot \
    -no-shutdown
fi
