#!/bin/bash
# Runner appele par `cargo run` — sortie serie uniquement (compatible SSH)
BINARY=$1
IMG=$(mktemp /tmp/kernel_test_XXXXXX.img)

# Pas de --only-section : `-O binary` recopie deja toutes les sections
# allouables dans l'ordre des adresses. Filtrer a la main faisait sauter
# .got / .data.rel.ro, dont les pointeurs etaient alors lus a zero.
objcopy -O binary "$BINARY" "${IMG}.bin"

dd if=/dev/zero of="$IMG" bs=512 count=8192 2>/dev/null
dd if=/home/kali/Day16/OS_Day16/boot.bin of="$IMG" conv=notrunc 2>/dev/null
dd if=/home/kali/Day16/OS_Day16/stage2.bin of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null
dd if="${IMG}.bin" of="$IMG" bs=512 seek=2 conv=notrunc 2>/dev/null

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
