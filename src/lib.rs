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

#[cfg(feature = "async")]
pub mod asynch;
pub mod clock;
pub mod clock_init;
pub mod delay;
pub mod dma;
pub mod efuse;
#[cfg(feature = "embassy")]
pub mod embassy;
pub mod gpio;
pub mod i2c;
pub mod i2s;
pub mod interrupt;
pub mod io_config;
pub mod lsadc;
pub mod macros;
pub mod peripherals;
pub mod prelude;
pub mod private;
pub mod pwm;
pub mod rtc;
pub mod safety;
pub mod sfc;
pub mod spi;
pub mod system;
pub mod tcxo;
pub mod time;
pub mod timer;
pub mod trng;
pub mod tsensor;
pub mod uart;
pub mod ulp_gpio;
pub mod wdt;

// Crypto modules
pub mod km;
pub mod pke;
pub mod spacc;

pub mod soc;

pub use peripherals::Peripherals;
pub use system::System;
