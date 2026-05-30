//! Watchdog Timer (WDT) driver for WS63.
//!
//! The WS63 WDT is a 24-bit down-counter that can generate an interrupt
//! or system reset when it reaches zero. The watchdog must be periodically
//! "fed" (restarted) to prevent a timeout.
//!
//! # Lock mechanism
//!
//! The WDT registers are protected by a lock. Write `0x5A5A5A5A` to
//! `WDT_LOCK` to unlock, any other value to lock. The driver handles
//! this automatically.
//!
//! # Operating modes
//!
//! - **Single-interrupt mode** (`wdt_mode = 0`): One interrupt before reset
//! - **Double-interrupt mode** (`wdt_mode = 1`): Two interrupts before reset
//!
//! # Clock source
//!
//! The WDT uses a 32.768 kHz clock. Timeout = load_value / 32768 seconds.

use crate::peripherals::Wdt;

/// WDT reset pulse length in clock cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetPulseLength {
    /// 2 clock cycles (~61 µs at 32.768 kHz)
    Cycles2 = 0,
    /// 4 clock cycles
    Cycles4 = 1,
    /// 8 clock cycles
    Cycles8 = 2,
    /// 16 clock cycles
    Cycles16 = 3,
    /// 32 clock cycles
    Cycles32 = 4,
    /// 64 clock cycles
    Cycles64 = 5,
    /// 128 clock cycles
    Cycles128 = 6,
    /// 256 clock cycles (~7.8 ms at 32.768 kHz)
    Cycles256 = 7,
}

/// WDT operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WdtMode {
    /// One interrupt, then reset on next timeout.
    SingleInterrupt,
    /// Two interrupts, then reset on third timeout.
    DoubleInterrupt,
}

/// Watchdog Timer driver.
pub struct Watchdog<'d> {
    _wdt: Wdt<'d>,
}

/// WDT clock frequency (32.768 kHz typical).
pub const WDT_CLOCK_HZ: u32 = 32_768;

/// Maximum WDT timeout value (24-bit).
pub const WDT_MAX_LOAD: u32 = 0x00FF_FFFF;

impl<'d> Watchdog<'d> {
    /// Create a new watchdog driver from the WDT peripheral.
    pub fn new(wdt: Wdt<'d>) -> Self {
        let wd = Self { _wdt: wdt };
        wd.unlock();
        wd
    }

    fn regs(&self) -> &'static ws63_pac::wdt::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Wdt::ptr() }
    }

    /// Unlock the WDT registers for configuration.
    fn unlock(&self) {
        unsafe {
            self.regs().wdt_lock().write(|w| w.bits(0x5A5A5A5A));
        }
    }

    /// Lock the WDT registers to prevent accidental modification.
    fn lock(&self) {
        unsafe {
            self.regs().wdt_lock().write(|w| w.bits(0x0000_0000));
        }
    }

    /// Configure and enable the watchdog.
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Timeout period in milliseconds.
    /// * `mode` - Interrupt mode (single or double interrupt before reset).
    /// * `reset_enable` - Whether to enable system reset on timeout.
    /// * `reset_pulse` - Reset pulse length if reset is enabled.
    pub fn configure(&mut self, timeout_ms: u32, mode: WdtMode, reset_enable: bool, reset_pulse: ResetPulseLength) {
        // Calculate load value from timeout in ms
        let load = ((timeout_ms as u64 * WDT_CLOCK_HZ as u64) / 1000) as u32;
        let load = load.min(WDT_MAX_LOAD);

        self.unlock();

        // Write load value (bits 8:31, 24-bit)
        unsafe {
            self.regs().wdt_load().write(|w| w.bits(load << 8));
        }

        // Configure control register
        let mut cr: u32 = 0;
        cr |= 0x01; // wdt_en = 1
        if reset_enable {
            cr |= 1 << 2; // rst_en
        }
        cr |= (reset_pulse as u32) << 3; // rst_pl
        cr |= 1 << 6; // wdt_imsk = 1 (mask interrupt initially)
        if matches!(mode, WdtMode::DoubleInterrupt) {
            cr |= 1 << 7; // wdt_mode = 1 (double interrupt)
        }

        unsafe {
            self.regs().wdt_cr().write(|w| w.bits(cr));
        }

        // Request counter value update
        self.request_counter();

        self.lock();
    }

    /// Enable the watchdog.
    pub fn enable(&mut self) {
        self.unlock();
        let cr = self.regs().wdt_cr().read().bits();
        unsafe {
            self.regs().wdt_cr().write(|w| w.bits(cr | 0x01));
        }
        self.lock();
    }

    /// Disable the watchdog.
    ///
    /// # Safety
    ///
    /// Disabling the watchdog may leave the system vulnerable to lock-ups.
    /// Only use this when you have an alternative safety mechanism.
    pub fn disable(&mut self) {
        self.unlock();
        let cr = self.regs().wdt_cr().read().bits();
        unsafe {
            self.regs().wdt_cr().write(|w| w.bits(cr & !0x01));
        }
        self.lock();
    }

    /// Feed (restart) the watchdog to prevent timeout.
    ///
    /// Writes any value other than `0x5A5A5A5A` to the restart register.
    pub fn feed(&mut self) {
        self.unlock();
        unsafe {
            self.regs().wdt_restart().write(|w| w.bits(0x0000_0001));
        }
        self.lock();
    }

    /// Read the current counter value.
    pub fn counter_value(&self) -> u32 {
        self.request_counter();
        self.regs().wdt_cnt().read().bits()
    }

    /// Request a counter value update.
    fn request_counter(&self) {
        unsafe {
            self.regs().wdt_ccvr_en().write(|w| w.bits(0x01));
        }
        while self.regs().wdt_ccvr_en().read().bits() & 0x02 == 0 {}
    }

    /// Check if a watchdog interrupt is pending (raw, unmasked).
    pub fn interrupt_pending(&self) -> bool {
        self.regs().wdt_raw_intr().read().bits() & 0x01 != 0
    }

    /// Check if a watchdog interrupt is pending (after mask).
    pub fn interrupt_masked(&self) -> bool {
        self.regs().wdt_intr().read().bits() & 0x01 != 0
    }

    /// Clear the watchdog interrupt (read from `WDT_EOI`).
    pub fn clear_interrupt(&self) {
        let _ = self.regs().wdt_eoi().read().bits();
    }

    /// Enable the watchdog interrupt (unmask).
    pub fn enable_interrupt(&mut self) {
        self.unlock();
        let cr = self.regs().wdt_cr().read().bits();
        unsafe {
            self.regs().wdt_cr().write(|w| w.bits(cr & !(1 << 6)));
        }
        self.lock();
    }

    /// Disable the watchdog interrupt (mask).
    pub fn disable_interrupt(&mut self) {
        self.unlock();
        let cr = self.regs().wdt_cr().read().bits();
        unsafe {
            self.regs().wdt_cr().write(|w| w.bits(cr | (1 << 6)));
        }
        self.lock();
    }

    /// Check if the watchdog is currently busy.
    pub fn is_busy(&self) -> bool {
        self.regs().wdt_status().read().bits() & 0x01 == 0
    }
}
