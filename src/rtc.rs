//! Real-Time Clock (RTC) driver for WS63.
//!
//! The WS63 RTC is a 48-bit free-running counter that can operate in
//! free-running or periodic mode. It can generate interrupts when the
//! counter matches a programmed load value.
//!
//! # Clock source
//!
//! The RTC runs from a 32.768 kHz clock.

use crate::peripherals::Rtc;

/// RTC operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcMode {
    /// Free-running mode: counter runs continuously.
    FreeRunning,
    /// Periodic mode: counter resets when it reaches the load value.
    Periodic,
}

/// Real-Time Clock driver.
pub struct RtcDriver<'d> {
    _rtc: Rtc<'d>,
}

/// RTC clock frequency (32.768 kHz).
pub const RTC_CLOCK_HZ: u32 = 32_768;

impl<'d> RtcDriver<'d> {
    /// Create a new RTC driver from the RTC peripheral.
    pub fn new(rtc: Rtc<'d>) -> Self {
        Self { _rtc: rtc }
    }

    fn regs(&self) -> &'static ws63_pac::rtc::RegisterBlock {
        unsafe { &*Rtc::ptr() }
    }

    /// Configure the RTC.
    ///
    /// # Arguments
    ///
    /// * `mode` - Operating mode (free-running or periodic).
    /// * `load_value` - In periodic mode, the counter resets when reaching this value.
    pub fn configure(&mut self, mode: RtcMode, load_value: u32) {
        unsafe {
            self.regs().rtc_load_count().write(|w| w.bits(load_value));
        }

        let mut ctrl: u32 = 0;
        ctrl |= 0x01; // enable
        if matches!(mode, RtcMode::Periodic) {
            ctrl |= 1 << 1; // mode = periodic
        }
        ctrl |= 1 << 2; // int_mask = masked initially

        unsafe {
            self.regs().rtc_control().write(|w| w.bits(ctrl));
        }
    }

    /// Enable the RTC counter.
    pub fn enable(&mut self) {
        let ctrl = self.regs().rtc_control().read().bits();
        unsafe {
            self.regs().rtc_control().write(|w| w.bits(ctrl | 0x01));
        }
    }

    /// Disable the RTC counter.
    pub fn disable(&mut self) {
        let ctrl = self.regs().rtc_control().read().bits();
        unsafe {
            self.regs().rtc_control().write(|w| w.bits(ctrl & !0x01));
        }
    }

    /// Set the load (alarm) value.
    pub fn set_load(&mut self, load_value: u32) {
        unsafe {
            self.regs().rtc_load_count().write(|w| w.bits(load_value));
        }
    }

    /// Read the current counter value (lower 32 bits of the 48-bit counter).
    pub fn current_value(&self) -> u32 {
        self.regs().rtc_current_value().read().bits()
    }

    /// Enable RTC interrupt (unmask).
    pub fn enable_interrupt(&mut self) {
        let ctrl = self.regs().rtc_control().read().bits();
        unsafe {
            self.regs().rtc_control().write(|w| w.bits(ctrl & !(1 << 2)));
        }
    }

    /// Disable RTC interrupt (mask).
    pub fn disable_interrupt(&mut self) {
        let ctrl = self.regs().rtc_control().read().bits();
        unsafe {
            self.regs().rtc_control().write(|w| w.bits(ctrl | (1 << 2)));
        }
    }

    /// Check if an RTC interrupt is pending.
    pub fn interrupt_pending(&self) -> bool {
        self.regs().rtc_int_status().read().bits() & 0x01 != 0
    }

    /// Clear the RTC interrupt.
    pub fn clear_interrupt(&self) {
        let _ = self.regs().rtc_eoi().read().bits();
    }
}
