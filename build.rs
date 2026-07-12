//! Build script for hisi-hal.
//!
//! The HAL itself needs no codegen — this script exists ONLY to put the two
//! linker scripts required by the on-target HIL integration test (`tests/hil.rs`,
//! an embedded-test ELF) on the link line. It is a no-op for the normal library
//! build and for the host unit-test build; it only emits link args when building
//! for the RISC-V target, so a plain `cargo build` / `cargo test --target
//! x86_64-unknown-linux-gnu` is byte-unchanged. (Mirrors tests-hil/build.rs,
//! which can emit them unconditionally because that crate is riscv-only.)
//!
//! Two linker scripts must be on the link line for an embedded-test ELF:
//!
//!   * `hisi-riscv-link.x` — hisi-riscv-rt's entry script (startup placement, memory
//!     map, device vectors, and — under WS63's `boot-header` feature, turned on
//!     by this crate's chip-ws63 feature — the 0x300 HiSilicon image header so
//!     the bare ELF is bootable). The entry-script *name* is `hisi-riscv-link.x` for
//!     BOTH chips: hisi-riscv-rt's build.rs always writes `hisi-riscv-link.x` (it only
//!     varies what it `INCLUDE`s), so this single chip-agnostic `-Thisi-riscv-link.x`
//!     flag is emitted for any riscv build. A library dependency's
//!     `rustc-link-arg` does NOT propagate to a downstream binary/test, so the
//!     test binary must request the `-T` itself; hisi-riscv-rt exports its
//!     OUT_DIR on the (propagating) link-search path so it resolves.
//!
//!   * `embedded-test.x` — embedded-test's fragment that keeps the
//!     EMBEDDED_TEST_VERSION marker + the `.embedded_test` test-case section.
//!     Provided on the link-search path by the `embedded-test-linker-script`
//!     crate (a riscv-only dev-dep). Used for both chips.
//!
//! These flags are harmless for the library/example link of a downstream riscv
//! crate too (those crates supply their own `-Thisi-riscv-link.x` already and a
//! repeated `-T` of the same on-search-path script is idempotent), but to be
//! safe they are scoped to this crate's own build only by the cargo build-script
//! contract (rustc-link-arg applies to THIS package's artifacts).
fn main() {
    // Only the RISC-V target builds the on-target test ELF; the host build (lib
    // unit tests, proptest) must not see these (the linker scripts + their search
    // paths only exist in the riscv graph, where the dev-deps are present).
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "riscv32" {
        // Entry script name is the same (`hisi-riscv-link.x`) for WS63 and BS21 — rt
        // only varies the INCLUDEd fragments, not the entry-script name.
        println!("cargo:rustc-link-arg=-Thisi-riscv-link.x");
        println!("cargo:rustc-link-arg=-Tembedded-test.x");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
