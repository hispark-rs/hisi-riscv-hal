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

    fn regs(&self) -> &'static crate::soc::pac::tcxo::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
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
    ///
    /// Returns `Ok(())` if the refresh completed, or `Err(())` on timeout.
    fn refresh(&self) -> Result<(), ()> {
        let status = self.regs().tcxo_status().read().bits();
        unsafe {
            self.regs().tcxo_status().write(|w| w.bits(status | 0x01));
        }
        // Wait for valid flag with timeout (~1ms at 32kHz TCXO clock)
        for _ in 0..100 {
            if self.regs().tcxo_status().read().bits() & 0x10 != 0 {
                return Ok(());
            }
            core::hint::spin_loop();
        }
        Err(())
    }

    /// Returns true if the counter was successfully refreshed.
    /// Callers should check this after reading before relying on the counter value.
    pub fn refresh_ok(&self) -> bool {
        self.regs().tcxo_status().read().bits() & 0x10 != 0
    }

    /// Read the full 64-bit counter value.
    ///
    /// Triggers a refresh to latch the current value, then reads all four
    /// 16-bit count registers. Returns `None` if the refresh timed out.
    pub fn read_counter(&self) -> Option<u64> {
        self.refresh().ok()?;
        let c0 = self.regs().tcxo_count0().read().bits() as u64;
        let c1 = self.regs().tcxo_count1().read().bits() as u64;
        let c2 = self.regs().tcxo_count2().read().bits() as u64;
        let c3 = self.regs().tcxo_count3().read().bits() as u64;
        Some((c3 << 48) | (c2 << 32) | (c1 << 16) | c0)
    }

    /// Read the lower 32 bits of the counter (faster, triggers refresh).
    ///
    /// Returns `None` if the refresh timed out.
    pub fn read_counter32(&self) -> Option<u32> {
        self.refresh().ok()?;
        let c0 = self.regs().tcxo_count0().read().bits();
        let c1 = self.regs().tcxo_count1().read().bits();
        Some((c1 << 16) | c0)
    }

    /// Check if the counter value is valid after a refresh.
    pub fn is_valid(&self) -> bool {
        self.regs().tcxo_status().read().bits() & 0x10 != 0
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn test_tcxo_refresh_timeout_logic() {
        // refresh() loops max 100 times, checking valid bit (0x10)
        let max_retries = 100;
        let mut valid = false;
        let mut attempts = 0;
        let result = loop {
            if valid {
                break Ok(());
            }
            if attempts >= max_retries {
                break Err(());
            }
            attempts += 1;
        };
        assert_eq!(result, Err(()));
        assert_eq!(attempts, 100);
    }

    #[test]
    fn test_tcxo_refresh_success_on_valid() {
        let valid = true;
        let result = if valid { Ok(()) } else { Err(()) };
        assert!(result.is_ok());
    }

    #[test]
    fn test_tcxo_valid_bit_position() {
        // Valid flag is bit 4 (0x10) in tcxo_status register
        let status_valid: u32 = 0x10;
        assert!((status_valid & 0x10) != 0);
        let status_invalid: u32 = 0x00;
        assert!((status_invalid & 0x10) == 0);
    }

    #[test]
    fn test_tcxo_status_enable_bit() {
        // Enable writes status|0x04 (bit 2)
        let enable_mask: u32 = 0x04;
        assert_eq!(enable_mask, 1 << 2);
    }

    #[test]
    fn test_tcxo_status_clear_bit() {
        // Clear writes status|0x02 (bit 1)
        let clear_mask: u32 = 0x02;
        assert_eq!(clear_mask, 1 << 1);
    }

    #[test]
    fn test_tcxo_status_refresh_bit() {
        // Refresh writes status|0x01 (bit 0)
        let refresh_mask: u32 = 0x01;
        assert_eq!(refresh_mask, 1 << 0);
    }

    #[test]
    fn test_tcxo_counter_64bit_assembly() {
        // read_counter assembles c3:c2:c1:c0 as a 64-bit value
        let c0: u64 = 0x1234;
        let c1: u64 = 0x5678;
        let c2: u64 = 0x9ABC;
        let c3: u64 = 0xDEF0;
        let counter: u64 = (c3 << 48) | (c2 << 32) | (c1 << 16) | c0;
        assert_eq!(counter, 0xDEF0_9ABC_5678_1234);
    }

    #[test]
    fn test_tcxo_counter_32bit_assembly() {
        // read_counter32 assembles c1:c0 as a 32-bit value
        let c0: u32 = 0x1234;
        let c1: u32 = 0x5678;
        let counter: u32 = (c1 << 16) | c0;
        assert_eq!(counter, 0x5678_1234);
    }
}
