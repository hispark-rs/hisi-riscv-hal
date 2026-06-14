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
//! The WDT counts at the **TCXO crystal clock** ([`WDT_CLOCK_HZ`] = 24 MHz on
//! 24 MHz-crystal boards) — NOT a 32.768 kHz RTC clock — programmed by the vendor
//! `watchdog_port_set_clock(REQ_24M)` in `clock_init`. The 24-bit `WDT_LOAD` field
//! lives in bits [31:8], so the effective counter has 256-cycle resolution
//! (`WDT_LOAD_RESEV = 8`) and the maximum timeout is `(0xFFFFFF << 8) / 24 MHz ≈ 178 s`.

use crate::peripherals::Wdt;

/// WDT reset pulse length in clock cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetPulseLength {
    /// 2 clock cycles (~83 ns at 24 MHz)
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
    /// 256 clock cycles (~10.7 µs at 24 MHz)
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

/// WDT counting clock = the TCXO crystal ([`crate::soc::chip::TCXO_HZ`], 24 MHz).
pub const WDT_CLOCK_HZ: u32 = crate::soc::chip::TCXO_HZ;

/// Maximum `WDT_LOAD` field value (24-bit; the field occupies `WDT_LOAD` bits [31:8]).
pub const WDT_MAX_LOAD: u32 = 0x00FF_FFFF;

/// Reserved low bits of `WDT_LOAD`: the 24-bit load field sits in bits [31:8], so
/// the total timeout cycle count is shifted right by this to form the field value
/// (matches the vendor `LOAD_RESEV = 8` in `hal_watchdog_v151`).
const WDT_LOAD_RESEV: u32 = 8;

impl<'d> Watchdog<'d> {
    /// Create a new watchdog driver from the WDT peripheral.
    pub fn new(wdt: Wdt<'d>) -> Self {
        let wd = Self { _wdt: wdt };
        wd.unlock();
        wd
    }

    fn regs(&self) -> &'static crate::soc::pac::wdt::RegisterBlock {
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
        // Timeout(ms) → total WDT clock cycles, then shift out the reserved low
        // 8 bits to form the 24-bit WDT_LOAD field. Matches the vendor
        // hal_watchdog_v151_set_attr: `cycles = timeout_s * clock; field = cycles >> LOAD_RESEV`.
        let cycles = (timeout_ms as u64 * WDT_CLOCK_HZ as u64) / 1000;
        // Saturate in u64 BEFORE narrowing: a multi-hour timeout shifts to a
        // value > u32::MAX, and casting to u32 first would TRUNCATE (wrap) it to
        // a bogus small load instead of clamping. Clamp in u64, then the
        // already-bounded result narrows losslessly. (The 24-bit field caps the
        // real timeout at ~178 s anyway, so anything larger must pin to the max.)
        let load = (cycles >> WDT_LOAD_RESEV).min(WDT_MAX_LOAD as u64) as u32;

        self.unlock();

        // Write the 24-bit load field into WDT_LOAD bits [31:8].
        unsafe {
            self.regs().wdt_load().write(|w| w.bits(load << WDT_LOAD_RESEV));
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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::{ResetPulseLength, WDT_CLOCK_HZ, WDT_LOAD_RESEV, WDT_MAX_LOAD, WdtMode};

    /// Re-derivation of the `configure()` timeout→field math: timeout(ms) is
    /// converted to total WDT clock cycles, shifted right by the reserved low
    /// bits to form the 24-bit field, then clamped to the max field value.
    /// This mirrors `Watchdog::configure` exactly (same types/rounding/clamp)
    /// without touching any MMIO register.
    fn load_field(timeout_ms: u32) -> u32 {
        let cycles = (timeout_ms as u64 * WDT_CLOCK_HZ as u64) / 1000;
        (cycles >> WDT_LOAD_RESEV).min(WDT_MAX_LOAD as u64) as u32
    }

    /// Re-derivation of the control-register encoding from `configure()`.
    fn control_bits(mode: WdtMode, reset_enable: bool, reset_pulse: ResetPulseLength) -> u32 {
        let mut cr: u32 = 0;
        cr |= 0x01; // wdt_en
        if reset_enable {
            cr |= 1 << 2; // rst_en
        }
        cr |= (reset_pulse as u32) << 3; // rst_pl
        cr |= 1 << 6; // wdt_imsk
        if matches!(mode, WdtMode::DoubleInterrupt) {
            cr |= 1 << 7; // wdt_mode
        }
        cr
    }

    #[test]
    fn reset_pulse_discriminants_are_the_3bit_field() {
        // The eight pulse lengths map to the contiguous 3-bit rst_pl field 0..=7.
        assert_eq!(ResetPulseLength::Cycles2 as u32, 0);
        assert_eq!(ResetPulseLength::Cycles4 as u32, 1);
        assert_eq!(ResetPulseLength::Cycles8 as u32, 2);
        assert_eq!(ResetPulseLength::Cycles16 as u32, 3);
        assert_eq!(ResetPulseLength::Cycles32 as u32, 4);
        assert_eq!(ResetPulseLength::Cycles64 as u32, 5);
        assert_eq!(ResetPulseLength::Cycles128 as u32, 6);
        assert_eq!(ResetPulseLength::Cycles256 as u32, 7);
    }

    #[test]
    fn zero_timeout_yields_zero_load() {
        // A 0 ms timeout produces a 0-cycle, 0-field load (no underflow/panic).
        assert_eq!(load_field(0), 0);
    }

    #[test]
    fn known_timeout_matches_hand_computed_field() {
        // 1000 ms = WDT_CLOCK_HZ cycles; the field is that shifted right by RESEV.
        let cycles = WDT_CLOCK_HZ as u64; // 1000 ms * clock / 1000
        let expected = (cycles >> WDT_LOAD_RESEV) as u32;
        assert_eq!(load_field(1000), expected);
        assert!(expected <= WDT_MAX_LOAD);
    }

    #[test]
    fn large_timeout_clamps_to_max_field() {
        // u32::MAX ms vastly exceeds the 24-bit field; the load saturates, never wraps.
        assert_eq!(load_field(u32::MAX), WDT_MAX_LOAD);
    }

    #[test]
    fn load_field_is_monotonic_until_clamp() {
        // More milliseconds never produce a smaller field (within the unclamped range).
        let a = load_field(1000);
        let b = load_field(2000);
        assert!(b >= a);
        assert!(b <= WDT_MAX_LOAD);
    }

    #[test]
    fn first_timeout_that_reaches_clamp() {
        // A timeout past the (WDT_MAX_LOAD+1)<<RESEV cycle boundary clamps to max.
        // Compute that boundary in ms with CEILING division (+1 ms margin) so the
        // input definitely clears the threshold — a floored value lands one tick
        // below it (cycles>>RESEV == WDT_MAX_LOAD-ε) and would not yet clamp.
        let threshold_cycles = (WDT_MAX_LOAD as u64 + 1) << WDT_LOAD_RESEV;
        let big_ms = (threshold_cycles * 1000).div_ceil(WDT_CLOCK_HZ as u64) + 1;
        // big_ms may exceed u32; saturate the *input* to u32 like the real API would receive.
        let ms = big_ms.min(u32::MAX as u64) as u32;
        assert_eq!(load_field(ms), WDT_MAX_LOAD);
    }

    #[test]
    fn control_bits_minimal_config() {
        // Single-interrupt, no reset, pulse 0: only wdt_en (bit0) + wdt_imsk (bit6).
        let cr = control_bits(WdtMode::SingleInterrupt, false, ResetPulseLength::Cycles2);
        assert_eq!(cr, 0x01 | (1 << 6));
    }

    #[test]
    fn control_bits_full_config() {
        // Double-interrupt + reset + pulse 256 (field 7): all encoded bits set.
        let cr = control_bits(WdtMode::DoubleInterrupt, true, ResetPulseLength::Cycles256);
        let expected = 0x01            // wdt_en
            | (1 << 2)                 // rst_en
            | (7 << 3)                 // rst_pl = Cycles256
            | (1 << 6)                 // wdt_imsk
            | (1 << 7); // wdt_mode (double)
        assert_eq!(cr, expected);
    }

    #[test]
    fn control_bits_reset_pulse_occupies_field() {
        // rst_pl lives in bits [5:3]; verify each variant lands there and nowhere else.
        for (variant, val) in
            [(ResetPulseLength::Cycles2, 0u32), (ResetPulseLength::Cycles16, 3), (ResetPulseLength::Cycles256, 7)]
        {
            let cr = control_bits(WdtMode::SingleInterrupt, false, variant);
            assert_eq!((cr >> 3) & 0x7, val);
        }
    }

    #[test]
    fn mode_only_affects_bit7() {
        // The only difference between single and double interrupt is wdt_mode (bit7).
        let single = control_bits(WdtMode::SingleInterrupt, true, ResetPulseLength::Cycles32);
        let double = control_bits(WdtMode::DoubleInterrupt, true, ResetPulseLength::Cycles32);
        assert_eq!(single ^ double, 1 << 7);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use super::{WDT_CLOCK_HZ, WDT_LOAD_RESEV, WDT_MAX_LOAD};
    use proptest::prelude::*;

    fn load_field(timeout_ms: u32) -> u32 {
        let cycles = (timeout_ms as u64 * WDT_CLOCK_HZ as u64) / 1000;
        (cycles >> WDT_LOAD_RESEV).min(WDT_MAX_LOAD as u64) as u32
    }

    proptest! {
        /// Fuzz: the load field is always within the 24-bit max for any u32 timeout.
        #[test]
        fn load_field_never_exceeds_max(ms in any::<u32>()) {
            prop_assert!(load_field(ms) <= WDT_MAX_LOAD);
        }

        /// Fuzz: the conversion never panics (no overflow) for any u32 timeout.
        #[test]
        fn load_field_never_panics(ms in any::<u32>()) {
            let _ = load_field(ms);
        }

        /// Fuzz: monotonic — a longer timeout never yields a smaller load field.
        #[test]
        fn load_field_is_monotonic(a in any::<u32>(), b in any::<u32>()) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(load_field(hi) >= load_field(lo));
        }
    }
}
