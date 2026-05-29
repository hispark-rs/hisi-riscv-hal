//! Clock configuration for WS63.
//!
//! The WS63 uses a CLDO_CRG (Clock and Reset Generator) for peripheral clock
//! enables, dividers, and clock source selection. The system boots at 240MHz
//! and this module provides helpers for enabling/disabling peripheral clocks.

use crate::peripherals::CldoCrg;
use crate::system::{Clocks, System};

/// Clock control peripheral wrapper.
pub struct ClockControl<'d> {
    cldo_crg: CldoCrg<'d>,
}

impl<'d> ClockControl<'d> {
    /// Configure the system clocks with defaults (240MHz system, 240MHz peripheral bus).
    pub fn configure_system(system: System<'d>) -> Self {
        Self { cldo_crg: system.cldo_crg }
    }

    /// Freeze the clock configuration, returning the resolved [`Clocks`].
    pub fn freeze(self) -> Clocks {
        Clocks::default()
    }

    /// Enable the clock gate for a specific UART instance.
    pub fn enable_uart(&self, uart_idx: usize) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl1().read();
        let bit = match uart_idx {
            0 => 18,
            1 => 19,
            2 => 20,
            _ => unreachable!(),
        };
        cken.cken_ctl1().write(|w| unsafe { w.bits(bits.bits() | (1 << bit)) });
    }

    /// Enable the clock gate for a specific I2C instance.
    pub fn enable_i2c(&self, i2c_idx: usize) {
        // I2C clock gates are in cken_ctl0 general clock enables
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        let bit = match i2c_idx {
            0 => 18,
            1 => 19,
            _ => unreachable!(),
        };
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << bit)) });
    }

    /// Enable the clock gate for the SPI peripheral.
    pub fn enable_spi(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl1().read();
        cken.cken_ctl1().write(|w| unsafe { w.bits(bits.bits() | (1 << 25)) });
    }

    /// Enable the clock gate for PWM.
    pub fn enable_pwm(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        // PWM clock gates: bits 2:10
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (0x1FF << 2)) });
    }

    /// Enable the clock gate for TIMER.
    pub fn enable_timer(&self) {
        let cken = self.cldo_crg.register_block();
        let bits0 = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits0.bits() | (1 << 21)) });
    }

    /// Enable the clock gate for the LSADC peripheral.
    pub fn enable_lsadc(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << 22)) });
    }

    /// Enable the clock gate for the TSENSOR peripheral.
    pub fn enable_tsensor(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << 23)) });
    }

    /// Enable the clock gate for the I2S peripheral.
    pub fn enable_i2s(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << 24)) });
    }

    /// Enable the clock gate for the DMA peripheral.
    pub fn enable_dma(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl1().read();
        cken.cken_ctl1().write(|w| unsafe { w.bits(bits.bits() | (1 << 22)) });
    }

    /// Enable the clock gate for the SDMA peripheral.
    pub fn enable_sdma(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl1().read();
        cken.cken_ctl1().write(|w| unsafe { w.bits(bits.bits() | (1 << 23)) });
    }

    /// Enable the clock gate for the SFC (SPI Flash Controller) peripheral.
    pub fn enable_sfc(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl1().read();
        cken.cken_ctl1().write(|w| unsafe { w.bits(bits.bits() | (1 << 24)) });
    }

    /// Enable the clock gate for the TRNG peripheral.
    pub fn enable_trng(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << 25)) });
    }

    /// Enable the clock gate for the security acceleration (SPACC/PKE/KM) peripherals.
    pub fn enable_security(&self) {
        let cken = self.cldo_crg.register_block();
        let bits = cken.cken_ctl0().read();
        cken.cken_ctl0().write(|w| unsafe { w.bits(bits.bits() | (1 << 26)) });
    }
}
