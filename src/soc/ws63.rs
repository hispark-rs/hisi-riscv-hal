//! WS63 chip-specific PAC re-export and configuration.
#![allow(dead_code)]

pub use ws63_pac::interrupt::ExternalInterrupt as Interrupt;

/// System clock frequency (240 MHz).
pub const SYSTEM_CLOCK_HZ: u32 = 240_000_000;

/// Number of GPIO pins (19: GPIO0[7:0] + GPIO1[15:8] + GPIO2[18:16]).
pub const GPIO_COUNT: usize = 19;

/// Number of ULP GPIO pins (8: GPIO107-114).
pub const ULP_GPIO_COUNT: usize = 8;

/// Number of UART instances.
pub const UART_COUNT: usize = 3;

/// Number of I2C instances.
pub const I2C_COUNT: usize = 2;

/// Number of SPI instances.
pub const SPI_COUNT: usize = 2;

/// Number of PWM channels.
pub const PWM_CHANNEL_COUNT: usize = 8;

/// Number of DMA channels (per controller).
pub const DMA_CHANNEL_COUNT: usize = 4;

/// Number of TIMER instances.
pub const TIMER_COUNT: usize = 3;

/// Number of LSADC channels.
pub const LSADC_CHANNEL_COUNT: usize = 6;

/// TCXO counter width in bits.
pub const TCXO_COUNTER_WIDTH: usize = 64;

/// RTC counter width in bits.
pub const RTC_COUNTER_WIDTH: usize = 48;
