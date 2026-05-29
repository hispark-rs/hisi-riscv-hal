//! # ws63-hal — Hardware Abstraction Layer for HiSilicon WS63 (RISC-V).
//!
//! GPIO, UART, I2C, SPI, PWM, Timer, Clock, Interrupt management,
//! WDT, RTC, LSADC, TSENSOR, DMA, TRNG, TCXO, I2S, SFC, EFUSE, IO_CONFIG.
#![no_std]
#![allow(non_camel_case_types)]
#![allow(rustdoc::broken_intra_doc_links)]

pub mod clock;
pub mod dma;
pub mod efuse;
pub mod gpio;
pub mod i2c;
pub mod i2s;
pub mod interrupt;
pub mod io_config;
pub mod lsadc;
pub mod peripherals;
pub mod prelude;
pub mod pwm;
pub mod rtc;
pub mod sfc;
pub mod spi;
pub mod system;
pub mod tcxo;
pub mod timer;
pub mod trng;
pub mod tsensor;
pub mod uart;
pub mod ulp_gpio;
pub mod wdt;

mod soc;

pub use peripherals::Peripherals;
pub use system::System;
