//! TCXO 64-bit Free-Running Counter driver for WS63.
//!
//! The WS63 TCXO provides a 64-bit free-running counter that can be used
//! for precise timing measurements. The counter value must be latched
//! before reading by triggering a refresh.
//!
//! # Counter read procedure
//!
//! 1. Write `refresh = 1` to latch the current count value
//! 2. Wait for `valid` flag to be set
//! 3. Read `count0..count3` to get the full 64-bit value

use crate::peripherals::Tcxo;

/// TCXO 64-bit counter driver.
pub struct TcxoDriver<'d> {
    _tcxo: Tcxo<'d>,
}

impl<'d> TcxoDriver<'d> {
    /// Create a new TCXO driver.
    pub fn new(tcxo: Tcxo<'d>) -> Self {
        Self { _tcxo: tcxo }
    }

    fn regs(&self) -> &'static ws63_pac::tcxo::RegisterBlock {
        unsafe { &*Tcxo::ptr() }
    }

    /// Enable the TCXO counter.
    pub fn enable(&mut self) {
        let status = self.regs().tcxo_status().read().bits();
        unsafe {
            self.regs().tcxo_status().write(|w| w.bits(status | 0x04));
        }
    }

    /// Disable the TCXO counter.
    pub fn disable(&mut self) {
        let status = self.regs().tcxo_status().read().bits();
        unsafe {
            self.regs().tcxo_status().write(|w| w.bits(status & !0x04));
        }
    }

    /// Clear the TCXO counter (reset to zero).
    pub fn clear(&mut self) {
        let status = self.regs().tcxo_status().read().bits();
        unsafe {
            self.regs().tcxo_status().write(|w| w.bits(status | 0x02));
        }
    }

    /// Latch the current counter value by triggering a refresh.
    fn refresh(&self) {
        let status = self.regs().tcxo_status().read().bits();
        unsafe {
            self.regs().tcxo_status().write(|w| w.bits(status | 0x01));
        }
        // Wait for valid flag
        while self.regs().tcxo_status().read().bits() & 0x10 == 0 {}
    }

    /// Read the full 64-bit counter value.
    ///
    /// Triggers a refresh to latch the current value, then reads all four
    /// 16-bit count registers.
    pub fn read_counter(&self) -> u64 {
        self.refresh();
        let c0 = self.regs().tcxo_count0().read().bits() as u64;
        let c1 = self.regs().tcxo_count1().read().bits() as u64;
        let c2 = self.regs().tcxo_count2().read().bits() as u64;
        let c3 = self.regs().tcxo_count3().read().bits() as u64;
        (c3 << 48) | (c2 << 32) | (c1 << 16) | c0
    }

    /// Read the lower 32 bits of the counter (faster, triggers refresh).
    pub fn read_counter32(&self) -> u32 {
        self.refresh();
        let c0 = self.regs().tcxo_count0().read().bits() as u32;
        let c1 = self.regs().tcxo_count1().read().bits() as u32;
        (c1 << 16) | c0
    }

    /// Check if the counter value is valid after a refresh.
    pub fn is_valid(&self) -> bool {
        self.regs().tcxo_status().read().bits() & 0x10 != 0
    }
}
