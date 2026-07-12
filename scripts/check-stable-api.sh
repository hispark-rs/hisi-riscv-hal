#!/usr/bin/env bash
# Generate/check the WS63 0.6 stable API snapshot with unstable doc items hidden.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SNAPSHOT="$ROOT/api/hisi-riscv-hal-0.6-stable.txt"
TOOLCHAIN="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$ROOT/rust-toolchain.toml")"
WORK="$(mktemp -d)"
ACTUAL="$WORK/actual.txt"
trap 'rm -rf "$WORK"' EXIT

if [[ -z "$TOOLCHAIN" ]]; then
    echo "could not read [toolchain].channel from rust-toolchain.toml" >&2
    exit 1
fi

command -v cargo-public-api >/dev/null 2>&1 || {
    echo "cargo-public-api is required (CI pins 0.52.0)" >&2
    exit 1
}

# Public API extraction is intentionally host-only: it compares Rust item paths
# and signatures, while the CI build/doc matrix separately proves the real
# riscv32imfc target. Run outside the repository tree so a parent workspace's
# .cargo/config.toml cannot redirect this inspection to the firmware target.
(
    cd "$WORK"
    RUSTFLAGS="--cfg instability_disable_unstable_docs" \
        cargo "+$TOOLCHAIN" public-api \
        --manifest-path "$ROOT/Cargo.toml" \
        --no-default-features \
        --features chip-ws63 \
        --color never \
        -sss
) > "$ACTUAL"

case "${1:---check}" in
    --check)
        diff -u "$SNAPSHOT" "$ACTUAL"
        ;;
    --update)
        mkdir -p "$(dirname "$SNAPSHOT")"
        cp "$ACTUAL" "$SNAPSHOT"
        echo "updated $SNAPSHOT"
        ;;
    *)
        echo "usage: $0 [--check|--update]" >&2
        exit 2
        ;;
esac
