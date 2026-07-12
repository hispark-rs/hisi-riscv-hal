#!/usr/bin/env bash
# Prove that the 0.7 package rename changes only the public crate path.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OLD="$ROOT/api/hisi-riscv-hal-0.6-stable.txt"
NEW="$ROOT/api/hisi-hal-0.7-stable.txt"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

sed 's/hisi_riscv_hal/hisi_hal/g' "$OLD" > "$TMP"
if ! diff -u "$TMP" "$NEW"; then
    echo "stable API changed beyond the hisi-riscv-hal -> hisi-hal crate rename" >&2
    exit 1
fi
echo "rename API parity: OK"
