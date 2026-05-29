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
pub use crate::clock::{ClockControl, Peripheral, PeripheralGuard};
pub use crate::delay::Delay;
pub use crate::dma::{Dma0, DmaDriver, Sdma0};
pub use crate::efuse::EfuseDriver;
pub use crate::gpio::{AnyPin, Flex, GpioPin, Input, InputConfig, InputMode, Output, OutputConfig, OutputMode, Pull};
pub use crate::i2c::I2c;
pub use crate::i2s::I2sDriver;
pub use crate::interrupt::{InterruptConfigurable, Priority};
pub use crate::io_config::IoConfigDriver;
pub use crate::lsadc::LsAdc;
pub use crate::pwm::PwmChannel;
pub use crate::rtc::RtcDriver;
pub use crate::sfc::SfcDriver;
pub use crate::spi::Spi;
pub use crate::system::System;
pub use crate::tcxo::TcxoDriver;
pub use crate::time::{Duration, Instant, Rate};
pub use crate::timer::{OneShotTimer, PeriodicTimer, TimerDriver};
pub use crate::trng::TrngDriver;
pub use crate::tsensor::TempSensor;
pub use crate::uart::Uart;
pub use crate::ulp_gpio::UlpGpioPin;
pub use crate::wdt::Watchdog;
