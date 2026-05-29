//! Prelude — re-exports commonly used traits and types.

pub use embedded_hal::digital::InputPin as _;
pub use embedded_hal::digital::OutputPin as _;
pub use embedded_hal::digital::StatefulOutputPin as _;
pub use embedded_hal::i2c::I2c as _;
pub use embedded_hal::spi::SpiBus as _;
pub use embedded_io::{Read as _, Write as _};
pub use nb::block;

pub use crate::Peripherals;
pub use crate::dma::{Dma0, DmaDriver, Sdma0};
pub use crate::efuse::EfuseDriver;
pub use crate::gpio::{GpioPin, Input, Output, Pull};
pub use crate::i2c::I2c;
pub use crate::i2s::I2sDriver;
pub use crate::io_config::IoConfigDriver;
pub use crate::lsadc::LsAdc;
pub use crate::pwm::PwmChannel;
pub use crate::rtc::RtcDriver;
pub use crate::spi::Spi;
pub use crate::tcxo::TcxoDriver;
pub use crate::timer::TimerDriver;
pub use crate::trng::TrngDriver;
pub use crate::tsensor::TempSensor;
pub use crate::uart::Uart;
pub use crate::ulp_gpio::UlpGpioPin;
pub use crate::wdt::Watchdog;
