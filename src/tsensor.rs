//! Temperature Sensor (TSENSOR) driver for WS63.
//!
//! The WS63 temperature sensor provides a 10-bit digital temperature code
//! covering approximately -40°C to +125°C. It supports automatic refresh,
//! high/low temperature thresholds, and over-temperature detection.
//!
//! # Temperature calculation
//!
//! Temperature code range: 114 to 896 (approximate).
//! Temperature (°C) can be derived from the code using chip-specific
//! calibration data. Without calibration, the raw code is provided.
//!
//! # Interrupts
//!
//! - Conversion done interrupt
//! - Out-of-threshold interrupt (temperature outside [low, high])
//! - Over-temperature interrupt

use crate::peripherals::Tsensor;

/// Temperature sensor driver.
pub struct TempSensor<'d> {
    _tsensor: Tsensor<'d>,
}

/// Temperature code range constants.
pub const TEMP_CODE_MIN: u16 = 114;
pub const TEMP_CODE_MAX: u16 = 896;

impl<'d> TempSensor<'d> {
    /// Create a new temperature sensor driver.
    pub fn new(tsensor: Tsensor<'d>) -> Self {
        Self { _tsensor: tsensor }
    }

    fn regs(&self) -> &'static ws63_pac::tsensor::RegisterBlock {
        unsafe { &*Tsensor::ptr() }
    }

    /// Enable the temperature sensor.
    pub fn enable(&mut self) {
        let r = self.regs();
        let ctrl = r.tsensor_ctrl().read().bits();
        unsafe {
            r.tsensor_ctrl().write(|w| w.bits(ctrl | 0x01));
        }
    }

    /// Disable the temperature sensor.
    pub fn disable(&mut self) {
        let r = self.regs();
        let ctrl = r.tsensor_ctrl().read().bits();
        unsafe {
            r.tsensor_ctrl().write(|w| w.bits(ctrl & !0x01));
        }
    }

    /// Set the operating mode.
    ///
    /// * `mode` — Mode value (0-3). The exact meaning depends on the chip
    ///   configuration.
    pub fn set_mode(&mut self, mode: u8) {
        let r = self.regs();
        let ctrl = r.tsensor_ctrl().read().bits();
        let ctrl = (ctrl & !0x06) | (((mode as u32) & 0x03) << 1);
        unsafe {
            r.tsensor_ctrl().write(|w| w.bits(ctrl));
        }
    }

    /// Trigger a single temperature reading (software start).
    ///
    /// Writes 1 to the start register to refresh the temperature value.
    pub fn start_conversion(&mut self) {
        unsafe {
            self.regs().tsensor_start().write(|w| w.bits(0x01));
        }
    }

    /// Check if a temperature reading is ready.
    pub fn data_ready(&self) -> bool {
        self.regs().tsensor_sts().read().bits() & 0x02 != 0
    }

    /// Read the raw temperature code (10-bit, range 114-896).
    ///
    /// Returns `None` if data is not ready.
    pub fn read_raw(&self) -> Option<u16> {
        if !self.data_ready() {
            return None;
        }
        let sts = self.regs().tsensor_sts().read().bits();
        Some(((sts >> 2) & 0x3FF) as u16)
    }

    /// Read the temperature code, blocking until ready.
    pub fn read_blocking(&self) -> u16 {
        while !self.data_ready() {}
        let sts = self.regs().tsensor_sts().read().bits();
        ((sts >> 2) & 0x3FF) as u16
    }

    /// Clear the temperature sensor status.
    pub fn clear_status(&mut self) {
        let r = self.regs();
        let sts = r.tsensor_sts().read().bits();
        unsafe {
            r.tsensor_sts().write(|w| w.bits(sts | 0x01));
        }
    }

    /// Set the high temperature limit.
    ///
    /// Interrupt triggers when temperature code exceeds this value.
    pub fn set_high_limit(&mut self, code: u16) {
        unsafe {
            self.regs()
                .tsensor_temp_high_limit()
                .write(|w| w.bits((code & 0x3FF) as u32));
        }
    }

    /// Set the low temperature limit.
    ///
    /// Interrupt triggers when temperature code falls below this value.
    pub fn set_low_limit(&mut self, code: u16) {
        unsafe {
            self.regs()
                .tsensor_temp_low_limit()
                .write(|w| w.bits((code & 0x3FF) as u32));
        }
    }

    /// Set the over-temperature threshold.
    pub fn set_over_temp_threshold(&mut self, code: u16, enable_interrupt: bool) {
        let mut val = (code & 0x3FF) as u32;
        if enable_interrupt {
            val |= 1 << 10; // over_temp_en
        }
        unsafe {
            self.regs().tsensor_over_temp().write(|w| w.bits(val));
        }
    }

    /// Enable specific temperature interrupts.
    ///
    /// * `done_int` — Conversion done interrupt
    /// * `out_thresh_int` — Out-of-threshold interrupt
    /// * `overtemp_int` — Over-temperature interrupt
    pub fn enable_interrupts(&mut self, done_int: bool, out_thresh_int: bool, overtemp_int: bool) {
        let mut val: u32 = 0;
        if done_int {
            val |= 0x01;
        }
        if out_thresh_int {
            val |= 0x02;
        }
        if overtemp_int {
            val |= 0x04;
        }
        unsafe {
            self.regs().tsensor_temp_int_en().write(|w| w.bits(val));
        }
    }

    /// Disable all temperature interrupts.
    pub fn disable_all_interrupts(&mut self) {
        unsafe {
            self.regs().tsensor_temp_int_en().write(|w| w.bits(0));
        }
    }

    /// Check interrupt status.
    ///
    /// Returns `(done, out_thresh, overtemp)`.
    pub fn interrupt_status(&self) -> (bool, bool, bool) {
        let sts = self.regs().tsensor_temp_int_sts().read().bits();
        ((sts & 0x01) != 0, (sts & 0x02) != 0, (sts & 0x04) != 0)
    }

    /// Clear specific temperature interrupts.
    pub fn clear_interrupts(&mut self, done: bool, out_thresh: bool, overtemp: bool) {
        let mut val: u32 = 0;
        if done {
            val |= 0x01;
        }
        if out_thresh {
            val |= 0x02;
        }
        if overtemp {
            val |= 0x04;
        }
        unsafe {
            self.regs().tsensor_temp_int_clr().write(|w| w.bits(val));
        }
    }

    /// Configure automatic temperature refresh.
    ///
    /// * `period` — Refresh period in 32kHz clock cycles.
    /// * `enable` — Enable auto refresh.
    pub fn configure_auto_refresh(&mut self, period: u16, enable: bool) {
        unsafe {
            self.regs()
                .tsensor_auto_refresh_period()
                .write(|w| w.bits(period as u32));
            self.regs()
                .tsensor_auto_refresh_cfg()
                .write(|w| w.bits(if enable { 1 } else { 0 }));
        }
    }

    /// Enable temperature calibration.
    pub fn enable_calibration(&mut self) {
        let ctrl1 = self.regs().tsensor_ctrl1().read().bits();
        unsafe {
            self.regs().tsensor_ctrl1().write(|w| w.bits(ctrl1 | 0x01));
        }
    }

    /// Disable temperature calibration.
    pub fn disable_calibration(&mut self) {
        let ctrl1 = self.regs().tsensor_ctrl1().read().bits();
        unsafe {
            self.regs().tsensor_ctrl1().write(|w| w.bits(ctrl1 & !0x01));
        }
    }
}
