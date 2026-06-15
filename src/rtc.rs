//! Real-Time Clock (RTC) driver for WS63.
//!
//! The WS63 RTC is a 48-bit free-running counter that can operate in
//! free-running or periodic mode. It can generate interrupts when the
//! counter matches a programmed load value.
//!
//! # Clock source
//!
//! The RTC runs from a 32.768 kHz clock.
//!
//! # Preconditions (board / analog)
//!
//! The 32.768 kHz domain comes from an **external crystal that many boards do not
//! populate**. If the crystal is absent its clock domain never comes up, the
//! counter never advances, and touching the RTC registers can stall the bus / drop
//! the debug link. There is no software guard for a missing crystal — it is a board
//! provisioning fact — so the on-silicon RTC test is gated behind the opt-in
//! `hil-rtc` feature (OFF by default). Treat a populated 32.768 kHz crystal as a
//! hard precondition before constructing an `RtcDriver`; with it absent, prefer the
//! `timer` (TCXO) driver instead.

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

// ── Pure control-word encoding ─────────────────────────────────────
//
// The RTC control register packs three flags into the low bits:
//   bit 0 = enable, bit 1 = mode (1 = periodic), bit 2 = int_mask (1 = masked).
// These are kept as named constants so the register pokes and the host tests
// agree on the exact bit layout.

/// Control register bit: enable the RTC counter.
const CTRL_ENABLE: u32 = 0x01;
/// Control register bit: periodic mode (cleared = free-running).
const CTRL_PERIODIC: u32 = 1 << 1;
/// Control register bit: interrupt mask (set = masked / disabled).
const CTRL_INT_MASK: u32 = 1 << 2;

/// Compute the initial control-register value for [`RtcDriver::configure`].
///
/// Pure helper (no MMIO): enables the counter, selects periodic vs.
/// free-running mode, and starts with the interrupt masked.
const fn control_bits(mode: RtcMode) -> u32 {
    let mut ctrl = CTRL_ENABLE;
    if matches!(mode, RtcMode::Periodic) {
        ctrl |= CTRL_PERIODIC;
    }
    ctrl |= CTRL_INT_MASK;
    ctrl
}

impl<'d> RtcDriver<'d> {
    /// Create a new RTC driver from the RTC peripheral.
    pub fn new(rtc: Rtc<'d>) -> Self {
        Self { _rtc: rtc }
    }

    fn regs(&self) -> &'static crate::soc::pac::rtc::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
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

        let ctrl = control_bits(mode);

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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::*;

    #[test]
    fn free_running_control_word() {
        // Free-running: enable + int-mask set, periodic bit clear → 0b101.
        let ctrl = control_bits(RtcMode::FreeRunning);
        assert_eq!(ctrl, CTRL_ENABLE | CTRL_INT_MASK);
        assert_eq!(ctrl, 0b101);
    }

    #[test]
    fn periodic_control_word() {
        // Periodic: all three low bits set → 0b111.
        let ctrl = control_bits(RtcMode::Periodic);
        assert_eq!(ctrl, CTRL_ENABLE | CTRL_PERIODIC | CTRL_INT_MASK);
        assert_eq!(ctrl, 0b111);
    }

    #[test]
    fn periodic_bit_is_the_only_difference() {
        // The mode flag toggles exactly bit 1 and nothing else.
        let free = control_bits(RtcMode::FreeRunning);
        let periodic = control_bits(RtcMode::Periodic);
        assert_eq!(free ^ periodic, CTRL_PERIODIC);
    }

    #[test]
    fn enable_always_set_and_interrupt_starts_masked() {
        // Both modes power up enabled with the interrupt masked.
        for mode in [RtcMode::FreeRunning, RtcMode::Periodic] {
            let ctrl = control_bits(mode);
            assert_ne!(ctrl & CTRL_ENABLE, 0, "enable bit must be set");
            assert_ne!(ctrl & CTRL_INT_MASK, 0, "interrupt must start masked");
        }
    }

    #[test]
    fn control_bits_use_distinct_nonoverlapping_flags() {
        // The three flags occupy distinct single bits in the low nibble.
        assert_eq!(CTRL_ENABLE & CTRL_PERIODIC, 0);
        assert_eq!(CTRL_ENABLE & CTRL_INT_MASK, 0);
        assert_eq!(CTRL_PERIODIC & CTRL_INT_MASK, 0);
        // No flag leaks above the low nibble that configure() writes.
        assert_eq!((CTRL_ENABLE | CTRL_PERIODIC | CTRL_INT_MASK) & !0b111, 0);
    }

    #[test]
    fn enable_disable_mask_round_trip() {
        // Re-derive the RMW arithmetic of enable/disable/interrupt methods:
        // setting then clearing a flag returns the original word, and the
        // operations are idempotent.
        let base = control_bits(RtcMode::Periodic);
        // disable() clears bit 0, enable() sets it back.
        assert_eq!((base & !CTRL_ENABLE) | CTRL_ENABLE, base);
        // enable_interrupt() clears the mask, disable_interrupt() sets it.
        assert_eq!((base & !CTRL_INT_MASK) | CTRL_INT_MASK, base);
        // Idempotency: setting an already-set bit is a no-op.
        assert_eq!(base | CTRL_ENABLE, base);
    }

    #[test]
    fn rtc_clock_is_32_768_khz() {
        // The RTC runs from the 32.768 kHz crystal: 2^15 ticks per second.
        assert_eq!(RTC_CLOCK_HZ, 32_768);
        assert_eq!(RTC_CLOCK_HZ, 1 << 15);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use super::{CTRL_ENABLE, CTRL_INT_MASK, CTRL_PERIODIC, RtcMode, control_bits};
    use proptest::prelude::*;

    // Strategy producing both RtcMode variants.
    fn any_mode() -> impl Strategy<Value = RtcMode> {
        prop_oneof![Just(RtcMode::FreeRunning), Just(RtcMode::Periodic)]
    }

    proptest! {
        /// Fuzz: the control word never sets a bit outside the documented low
        /// nibble (enable|periodic|int_mask), for either mode. A stray high bit
        /// would corrupt the register write in `configure()`.
        #[test]
        fn control_word_has_no_stray_bits(mode in any_mode()) {
            let ctrl = control_bits(mode);
            prop_assert_eq!(ctrl & !(CTRL_ENABLE | CTRL_PERIODIC | CTRL_INT_MASK), 0);
        }

        /// Fuzz: enable and int_mask are unconditionally set in every mode
        /// (power-up enabled, interrupt masked) — independent of the mode flag.
        #[test]
        fn enable_and_mask_always_set(mode in any_mode()) {
            let ctrl = control_bits(mode);
            prop_assert_ne!(ctrl & CTRL_ENABLE, 0);
            prop_assert_ne!(ctrl & CTRL_INT_MASK, 0);
        }

        /// Fuzz: the periodic bit (and only it) reflects the mode — set iff
        /// Periodic, clear iff FreeRunning, with all other bits identical.
        #[test]
        fn periodic_bit_tracks_mode(mode in any_mode()) {
            let ctrl = control_bits(mode);
            let want_periodic = matches!(mode, RtcMode::Periodic);
            prop_assert_eq!(ctrl & CTRL_PERIODIC != 0, want_periodic);
            // Masking out the mode flag yields the same base word for both modes.
            prop_assert_eq!(ctrl & !CTRL_PERIODIC, CTRL_ENABLE | CTRL_INT_MASK);
        }

        /// Fuzz: encoding is total/deterministic — never panics and is stable
        /// across repeated calls for any mode input.
        #[test]
        fn control_bits_is_deterministic(mode in any_mode()) {
            prop_assert_eq!(control_bits(mode), control_bits(mode));
        }

        /// Fuzz: RMW round-trip — clearing then re-setting any single control
        /// flag (the arithmetic of enable/disable/{enable,disable}_interrupt)
        /// returns the original word, mirroring the driver's `ctrl & !bit | bit`.
        #[test]
        fn flag_clear_then_set_round_trips(mode in any_mode(), bits in any::<u32>()) {
            // Re-derive against an arbitrary register word, not just the encoder
            // output, to exercise the RMW logic over the full 32-bit space.
            let base = control_bits(mode) | bits;
            for flag in [CTRL_ENABLE, CTRL_PERIODIC, CTRL_INT_MASK] {
                // If the flag was set, clear+set restores it; if it was clear,
                // set+clear restores it. Either way the word is recovered.
                let had = base & flag != 0;
                if had {
                    prop_assert_eq!((base & !flag) | flag, base);
                } else {
                    prop_assert_eq!((base | flag) & !flag, base);
                }
            }
        }

        /// Fuzz: setting an already-set flag is idempotent; clearing an
        /// already-clear flag is idempotent (the enable/disable methods rely on
        /// this — calling enable() twice must not change the word).
        #[test]
        fn flag_ops_are_idempotent(bits in any::<u32>()) {
            for flag in [CTRL_ENABLE, CTRL_PERIODIC, CTRL_INT_MASK] {
                let set = bits | flag;
                prop_assert_eq!(set | flag, set);
                let cleared = bits & !flag;
                prop_assert_eq!(cleared & !flag, cleared);
            }
        }
    }
}
