# Changelog

All notable changes to ws63-hal will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **WDT driver** (`wdt.rs`): Watchdog timer with lock/unlock, interrupt/reset modes, configurable timeout
- **RTC driver** (`rtc.rs`): Real-Time Clock with 48-bit counter, free-running and periodic modes
- **TCXO driver** (`tcxo.rs`): 64-bit free-running counter with latched reads
- **LSADC driver** (`lsadc.rs`): Low-Speed ADC (12-bit SAR, 6 channels, CIC filter, offset/gain correction)
- **TSENSOR driver** (`tsensor.rs`): Temperature sensor (10-bit code, high/low thresholds, auto-refresh)
- **DMA driver** (`dma.rs`): DMA + SDMA controllers (4 channels each, scatter-gather, burst config)
- **TRNG driver** (`trng.rs`): True Random Number Generator (FRO entropy, FIFO read)
- **I2S driver** (`i2s.rs`): I2S/PCM audio interface (master/slave, 2-8 channels, clock dividers)
- **SFC driver** (`sfc.rs`): SPI Flash Controller (Standard/Dual/Quad SPI, Bus DMA, command operations)
- **IO_CONFIG driver** (`io_config.rs`): Pin mux and pad control (drive strength, pull, Schmitt trigger)
- **EFUSE driver** (`efuse.rs`): eFuse OTP controller (read/write control, status, AVDD control)
- **ULP_GPIO driver** (`ulp_gpio.rs`): Ultra-Low-Power GPIO (8 pins, full embedded-hal digital traits)
- **time module** (`time.rs`): `Instant`, `Duration`, `Rate` types (TCXO-backed)
- **delay module** (`delay.rs`): `Delay` driver implementing `embedded_hal::delay::DelayNs`
- **macros module** (`macros.rs`): `unstable_module!`, `any_peripheral!`, `infallible!`
- **private module** (`private.rs`): Sealed traits (`Sealed`, `DmaWord`, `DriverMode`, `Blocking`, `Async`)
- **SPACC driver** (`spacc.rs`): Security accelerator stub (AES/SM4/LEA/TDES/HASH/HMAC)
- **PKE driver** (`pke.rs`): Public Key Engine stub (RSA/ECC)
- **KM driver** (`km.rs`): Key Management stub (KLAD, keyslot locking)
- `embedded_hal::i2c::I2c` trait for I2C0 and I2C1
- `embedded_hal::spi::SpiBus` trait for SPI1
- `embedded_hal::pwm::SetDutyCycle` trait for PwmChannel
- `embedded_hal_nb::serial::Read/Write` traits for UART0/1/2
- `embedded_io::Read/Write` traits for UART1 and UART2
- `Peripheral` enum and `PeripheralGuard` RAII clock management with reference counting
- `DmaEligible` trait, `DmaChannelFor` trait, `DmaPeripheral` enum for DMA-peripheral binding
- `InputConfig`/`OutputConfig` structs for GPIO pin configuration
- `Flex` pin driver (combined input+output with embedded-hal traits)
- `AnyPin` type-erased GPIO pin
- `InputSignal`/`OutputSignal` types for peripheral interconnect
- `PeripheralOutput`/`PeripheralInput` sealed traits
- `OneShotTimer` and `PeriodicTimer` wrappers
- `embedded_hal::delay::DelayNs` for `OneShotTimer`
- `Delay` driver with `embedded_hal::delay::DelayNs`
- `ResetReason` enum and `software_reset()` in system module
- `Priority` levels, `InterruptConfigurable` trait, `InterruptHandler` struct
- `disable_peripheral()` and `reset_peripheral()` clock control methods
- `read_raw()` and `EfuseStatus::boot_complete()` eFuse methods
- Inline unit tests in `time.rs`

### Changed

- **GPIO**: Added `Input`/`Output`/`Flex` driver types alongside legacy `GpioPin<MODE>`
- **Timer**: Enhanced with `OneShotTimer` and `PeriodicTimer` wrapper types
- **Clock**: Refactored with private `write_cken_bit()` helper, eliminating duplicated register dispatch
- **prelude**: Expanded from 8 to 27 re-exports
- **soc/ws63**: Added `ULP_GPIO_COUNT`, `LSADC_CHANNEL_COUNT`, `TCXO_COUNTER_WIDTH`, `RTC_COUNTER_WIDTH`
- Made `soc` module and `Peripheral::cken_info()` public for testability
- formatted entire codebase with `cargo fmt`

### Fixed

- `DmaChannelFor` blanket impl removed (provided zero type safety)
- Duplicate `read_chip_id()` removed from efuse (alias for existing `read_control_data()`)
- Clock register dispatch deduplicated (15 methods → shared helper)
