#!/bin/bash
BINARY=$1
IMG=$(mktemp /tmp/kernel_test_XXXXXX.img)

# Convertit ELF en binaire plat
objcopy -O binary "$BINARY" "${IMG}.bin"

# Crée image disque avec boot sector du projet principal
dd if=/dev/zero of="$IMG" bs=512 count=2048 2>/dev/null
dd if=/home/kali/Day6/OS_Day6/boot.bin of="$IMG" conv=notrunc 2>/dev/null
dd if="${IMG}.bin" of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null

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
