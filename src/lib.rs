//! # ws63-hal — Hardware Abstraction Layer for HiSilicon WS63 (RISC-V).
//!
//! A comprehensive HAL providing safe, idiomatic Rust APIs for all WS63
//! peripherals. Modeled on esp-hal patterns with type-state GPIO, RAII
//! clock guards, DMA typing, and embedded-hal trait implementations.
//!
//! ## Clock gating
//!
//! Most peripherals need their CLDO_CRG clock gate enabled before register
//! access. The gates default to enabled out of reset; `clock_init::init_clocks()`
//! sets up the system clocks for firmware that does not boot through flashboot.
//! Constructors like `I2c::new_i2c0()`, `Uart::new_uart0()`, and `Watchdog::new()`
//! write configuration registers immediately. WDT/RTC/TCXO are always-on.
//!
//! ```ignore
//! let clocks = clock_init::init_clocks(&system.sys_ctl0, &system.cldo_crg);
//! // Now safe to construct peripheral drivers
//! let uart = Uart::new_uart0(peripherals.UART0, Config::default());
//! ```
// `no_std` for firmware builds; `std` is linked under `cfg(test)` so the host
// unit tests can use the standard test harness (run via `cargo test --target x86_64`).
#![cfg_attr(not(test), no_std)]
#![allow(non_camel_case_types)]
#![allow(rustdoc::broken_intra_doc_links)]

// Exactly one chip must be selected (each pulls its PAC + soc/<chip>.rs).
#[cfg(all(feature = "chip-ws63", feature = "chip-bs21"))]
compile_error!("ws63-hal: select exactly one chip — chip-ws63 (default) OR chip-bs21");
#[cfg(not(any(feature = "chip-ws63", feature = "chip-bs21")))]
compile_error!(
    "ws63-hal: no chip selected — enable chip-ws63 (default) or chip-bs21 \
     (e.g. --no-default-features --features chip-bs21)"
);

// ── Chip-neutral core (compiles for every chip) ────────────────────────────
pub mod delay;
pub mod gpio;
pub mod interrupt;
pub mod io_config;
pub mod macros;
pub mod peripherals;
pub mod prelude;
pub mod private;
pub mod soc;
pub mod system;
pub mod time;
pub mod timer;
pub mod tcxo;
pub mod uart;

// ── WS63-specific / not-yet-ported-to-BS21 drivers ──────────────────────────
// Gated to chip-ws63 for now: they touch WS63-only peripherals (Wi-Fi/RF, the
// full crypto block, SFC) or WS63-specific CRG/clock registers. BS21 ports land
// in later milestones; gating keeps the BS21 build to the milestone-1 subset
// while leaving the WS63 build (the default) byte-identical.
#[cfg(all(feature = "chip-ws63", feature = "async"))]
pub mod asynch;
#[cfg(feature = "chip-ws63")]
pub mod clock;
#[cfg(feature = "chip-ws63")]
pub mod clock_init;
#[cfg(feature = "chip-ws63")]
pub mod dma;
#[cfg(feature = "chip-ws63")]
pub mod efuse;
#[cfg(all(feature = "chip-ws63", feature = "embassy"))]
pub mod embassy;
#[cfg(feature = "chip-ws63")]
pub mod i2c;
#[cfg(feature = "chip-ws63")]
pub mod i2s;
#[cfg(feature = "chip-ws63")]
pub mod lsadc;
#[cfg(feature = "chip-ws63")]
pub mod pwm;
#[cfg(feature = "chip-ws63")]
pub mod rtc;
#[cfg(feature = "chip-ws63")]
pub mod safety;
#[cfg(feature = "chip-ws63")]
pub mod sfc;
#[cfg(feature = "chip-ws63")]
pub mod spi;
#[cfg(feature = "chip-ws63")]
pub mod trng;
#[cfg(feature = "chip-ws63")]
pub mod tsensor;
#[cfg(feature = "chip-ws63")]
pub mod ulp_gpio;
#[cfg(feature = "chip-ws63")]
pub mod wdt;
#[cfg(feature = "chip-ws63")]
pub mod km;
#[cfg(feature = "chip-ws63")]
pub mod pke;
#[cfg(feature = "chip-ws63")]
pub mod spacc;

pub use peripherals::Peripherals;
pub use system::System;
