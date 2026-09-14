#!/bin/bash
# Ecrit la taille reelle du kernel dans le MBR (offsets 500 et 504).
# Usage : patch_boot_size.sh boot.bin octets secteurs
set -e
BOOT=$1
BYTES=$2
SECTORS=$3

python3 - "$BOOT" "$BYTES" "$SECTORS" <<'PY'
import struct, sys
path, nbytes, nsec = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
with open(path, "r+b") as f:
    f.seek(500)
    f.write(struct.pack("<IH", nbytes, nsec))
PY
