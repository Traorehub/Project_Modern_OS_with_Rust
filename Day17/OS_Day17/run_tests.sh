#!/bin/bash
# Runner appele par `cargo run` : sortie serie uniquement (compatible SSH)
set -e
BINARY=$1
HERE=/home/kali/Day17/OS_Day17
IMG=$(mktemp /tmp/kernel_test_XXXXXX.img)

objcopy -O binary "$BINARY" "${IMG}.bin"

BYTES=$(stat -c%s "${IMG}.bin")
SECTORS=$(( (BYTES + 511) / 512 ))
cp "$HERE/boot.bin" "${IMG}.boot"
"$HERE/patch_boot_size.sh" "${IMG}.boot" "$BYTES" "$SECTORS"

dd if=/dev/zero of="$IMG" bs=512 count=8192 2>/dev/null
dd if="${IMG}.boot" of="$IMG" conv=notrunc 2>/dev/null
dd if="$HERE/stage2.bin" of="$IMG" bs=512 seek=1 conv=notrunc 2>/dev/null
dd if="${IMG}.bin" of="$IMG" bs=512 seek=2 conv=notrunc 2>/dev/null

qemu-system-x86_64 \
  -drive format=raw,file="$IMG" \
  -serial stdio \
  -display none \
  -no-reboot \
  -no-shutdown \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04

EXIT=$?
rm -f "$IMG" "${IMG}.bin" "${IMG}.boot"
exit $EXIT
