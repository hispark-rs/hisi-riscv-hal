//! Prelude — re-exports commonly used traits and types.

pub use embedded_hal::delay::DelayNs as _;
pub use embedded_hal::digital::InputPin as _;
pub use embedded_hal::digital::OutputPin as _;
pub use embedded_hal::digital::StatefulOutputPin as _;
pub use embedded_hal::i2c::I2c as _;
pub use embedded_hal::spi::SpiBus as _;
pub use embedded_io::{Read as _, Write as _};
pub use nb::block;

pub use crate::Peripherals;
pub use crate::delay::Delay;
pub use crate::gpio::{
    AnyPin, Flex, GpioPin, Input, InputConfig, InputMode, InterruptTrigger, Output, OutputConfig, OutputMode, Pull,
};
pub use crate::interrupt::{Interrupt, Priority};
pub use crate::tcxo::TcxoDriver;
pub use crate::time::{Duration, Instant, Rate};
pub use crate::timer::{OneShotTimer, PeriodicTimer, TimerDriver};
pub use crate::uart::Uart;
#[cfg(feature = "chip-ws63")]
pub use crate::{io_config::IoConfigDriver, system::System};

// WS63-only drivers (gated to chip-ws63 until ported to BS21).
#[cfg(feature = "chip-ws63")]
pub use crate::clock::Peripheral;
#[cfg(feature = "chip-ws63")]
pub use crate::dma::{Dma0, DmaDriver, Sdma0};
#[cfg(feature = "chip-ws63")]
pub use crate::efuse::EfuseDriver;
#[cfg(feature = "chip-ws63")]
pub use crate::i2c::I2c;
#[cfg(feature = "chip-ws63")]
pub use crate::i2s::I2sDriver;
#[cfg(feature = "chip-ws63")]
pub use crate::lsadc::LsAdc;
#[cfg(feature = "chip-ws63")]
pub use crate::pwm::PwmChannel;
#[cfg(feature = "chip-ws63")]
pub use crate::rtc::RtcDriver;
#[cfg(feature = "chip-ws63")]
pub use crate::sfc::SfcDriver;
#[cfg(feature = "chip-ws63")]
pub use crate::spi::Spi;
#[cfg(feature = "chip-ws63")]
pub use crate::trng::TrngDriver;
#[cfg(feature = "chip-ws63")]
pub use crate::tsensor::TempSensor;
#[cfg(feature = "chip-ws63")]
pub use crate::ulp_gpio::UlpGpioPin;
#[cfg(feature = "chip-ws63")]
pub use crate::wdt::Watchdog;
