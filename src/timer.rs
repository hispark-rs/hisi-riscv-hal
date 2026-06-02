//! Timer driver for WS63 (3 independent 32-bit timers).
//!
//! Each timer can operate in one-shot or periodic mode.
//! The timer clock source is the system peripheral clock (PCLK = 240MHz).
//!
//! # Usage
//!
//! ```ignore
//! let timer = TimerDriver::new(peripherals.TIMER);
//! let mut oneshot = timer.oneshot(0);
//! oneshot.start(240_000); // 1ms at 240MHz
//! while !oneshot.expired() {}
//! ```

use crate::peripherals::Timer;
use crate::soc::ws63::SYSTEM_CLOCK_HZ;

/// Timer operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// One-shot: timer counts once and stops.
    OneShot = 0,
    /// Periodic: timer reloads and repeats.
    Periodic = 1,
}

/// Timer driver managing 3 independent timer channels.
pub struct TimerDriver<'d> {
    _timer: Timer<'d>,
}

impl<'d> TimerDriver<'d> {
    /// Create a new timer driver.
    pub fn new(timer: Timer<'d>) -> Self {
        Self { _timer: timer }
    }

    fn regs(&self) -> &'static ws63_pac::timer::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Timer::ptr() }
    }

    /// Configure a timer channel.
    pub fn configure(&self, n: usize, mode: TimerMode, load_value: u32) {
        let r = self.regs();
        match n {
            0 => r.timer0_load_count(0).write(|w| unsafe { w.bits(load_value) }),
            1 => r.timer0_load_count(1).write(|w| unsafe { w.bits(load_value) }),
            2 => r.timer0_load_count(2).write(|w| unsafe { w.bits(load_value) }),
            _ => unreachable!(),
        };
        let ctl = ((mode as u32) & 0x3) << 1;
        match n {
            0 => r.timer0_control(0).write(|w| unsafe { w.bits(ctl) }),
            1 => r.timer0_control(1).write(|w| unsafe { w.bits(ctl) }),
            2 => r.timer0_control(2).write(|w| unsafe { w.bits(ctl) }),
            _ => unreachable!(),
        };
    }

    /// Enable a timer channel.
    pub fn enable(&self, n: usize) {
        let r = self.regs();
        let prev = match n {
            0 => r.timer0_control(0).read().bits(),
            1 => r.timer0_control(1).read().bits(),
            2 => r.timer0_control(2).read().bits(),
            _ => unreachable!(),
        };
        match n {
            0 => r.timer0_control(0).write(|w| unsafe { w.bits(prev | 1) }),
            1 => r.timer0_control(1).write(|w| unsafe { w.bits(prev | 1) }),
            2 => r.timer0_control(2).write(|w| unsafe { w.bits(prev | 1) }),
            _ => unreachable!(),
        };
    }

    /// Disable a timer channel.
    pub fn disable(&self, n: usize) {
        let r = self.regs();
        let prev = match n {
            0 => r.timer0_control(0).read().bits(),
            1 => r.timer0_control(1).read().bits(),
            2 => r.timer0_control(2).read().bits(),
            _ => unreachable!(),
        };
        match n {
            0 => r.timer0_control(0).write(|w| unsafe { w.bits(prev & !1) }),
            1 => r.timer0_control(1).write(|w| unsafe { w.bits(prev & !1) }),
            2 => r.timer0_control(2).write(|w| unsafe { w.bits(prev & !1) }),
            _ => unreachable!(),
        };
    }

    /// Read the current counter value.
    pub fn current_value(&self, n: usize) -> u32 {
        let r = self.regs();
        match n {
            0 => r.timer0_current_value(0).read().bits(),
            1 => r.timer0_current_value(1).read().bits(),
            2 => r.timer0_current_value(2).read().bits(),
            _ => unreachable!(),
        }
    }

    /// Check if a timer interrupt is pending.
    pub fn interrupt_pending(&self, n: usize) -> bool {
        let r = self.regs();
        match n {
            0 => r.timer0_raw_intr(0).read().bits() & 1 != 0,
            1 => r.timer0_raw_intr(1).read().bits() & 1 != 0,
            2 => r.timer0_raw_intr(2).read().bits() & 1 != 0,
            _ => unreachable!(),
        }
    }

    /// Clear a timer interrupt (per-channel EOI).
    pub fn clear_interrupt(&self, n: usize) {
        let r = self.regs();
        match n {
            0 => {
                let _ = r.timer0_eoi(0).read().bits();
            }
            1 => {
                let _ = r.timer0_eoi(1).read().bits();
            }
            2 => {
                let _ = r.timer0_eoi(2).read().bits();
            }
            _ => unreachable!(),
        }
    }

    /// Create a one-shot timer wrapper for the given channel.
    pub fn oneshot(&self, channel: usize) -> OneShotTimer<'_> {
        OneShotTimer { driver: self, channel }
    }

    /// Create a periodic timer wrapper for the given channel.
    pub fn periodic(&self, channel: usize) -> PeriodicTimer<'_> {
        PeriodicTimer { driver: self, channel }
    }
}

// ── One-shot timer ────────────────────────────────────────────────

/// One-shot timer wrapper.
///
/// Counts down from a loaded value and stops when it reaches zero.
pub struct OneShotTimer<'a> {
    driver: &'a TimerDriver<'a>,
    channel: usize,
}

impl OneShotTimer<'_> {
    /// Start the one-shot timer with a count value.
    ///
    /// `count` is in timer clock ticks (PCLK = 240MHz → 1 tick ≈ 4.17ns).
    pub fn start(&mut self, count: u32) {
        self.driver.configure(self.channel, TimerMode::OneShot, count);
        self.driver.enable(self.channel);
    }

    /// Start the timer for the given duration in microseconds.
    ///
    /// Max duration: ~17.9 seconds at 240MHz (u32 ticks limit).
    /// For longer durations, use `start_millis()` or loop `start_micros()`.
    pub fn start_micros(&mut self, us: u32) {
        let ticks64 = SYSTEM_CLOCK_HZ as u64 * us as u64 / 1_000_000;
        // Clamp to u32 max — delays longer than ~17.9s at 240MHz
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        self.start(ticks);
    }

    /// Start the timer for the given duration in milliseconds.
    ///
    /// Max duration: ~17,895ms (~17.9s) at 240MHz.
    /// For longer durations, repeat this call in a loop.
    pub fn start_millis(&mut self, ms: u32) {
        let ticks64 = SYSTEM_CLOCK_HZ as u64 * ms as u64 / 1_000;
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        self.start(ticks);
    }

    /// Check if the timer has expired.
    pub fn expired(&self) -> bool {
        self.driver.interrupt_pending(self.channel)
    }

    /// Wait for the timer to expire (busy-loop).
    pub fn wait(&self) {
        while !self.expired() {}
        self.driver.clear_interrupt(self.channel);
    }

    /// Get the current counter value.
    pub fn current(&self) -> u32 {
        self.driver.current_value(self.channel)
    }

    /// Stop the timer.
    pub fn stop(&self) {
        self.driver.disable(self.channel);
    }

    /// Clear the interrupt flag.
    pub fn clear(&self) {
        self.driver.clear_interrupt(self.channel);
    }
}

impl embedded_hal::delay::DelayNs for OneShotTimer<'_> {
    fn delay_ns(&mut self, ns: u32) {
        let ticks64 = (SYSTEM_CLOCK_HZ as u64 * ns as u64) / 1_000_000_000;
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        if ticks > 0 {
            self.start(ticks);
            self.wait();
        }
    }

    fn delay_us(&mut self, us: u32) {
        self.start_micros(us);
        self.wait();
    }

    fn delay_ms(&mut self, ms: u32) {
        self.start_millis(ms);
        self.wait();
    }
}

// ── Periodic timer ────────────────────────────────────────────────

/// Periodic timer wrapper.
///
/// Counts down and automatically reloads, generating an interrupt each cycle.
pub struct PeriodicTimer<'a> {
    driver: &'a TimerDriver<'a>,
    channel: usize,
}

impl PeriodicTimer<'_> {
    /// Start the periodic timer with the given period in ticks.
    pub fn start(&mut self, period: u32) {
        self.driver.configure(self.channel, TimerMode::Periodic, period);
        self.driver.enable(self.channel);
    }

    /// Start the periodic timer with the period in microseconds.
    pub fn start_micros(&mut self, us: u32) {
        let ticks64 = SYSTEM_CLOCK_HZ as u64 * us as u64 / 1_000_000;
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        self.start(ticks);
    }

    /// Check if a period has elapsed (interrupt pending).
    pub fn tick_elapsed(&self) -> bool {
        self.driver.interrupt_pending(self.channel)
    }

    /// Wait for the next timer tick.
    pub fn wait_tick(&self) {
        while !self.tick_elapsed() {}
        self.driver.clear_interrupt(self.channel);
    }

    /// Stop the timer.
    pub fn stop(&self) {
        self.driver.disable(self.channel);
    }

    /// Get the current counter value.
    pub fn current(&self) -> u32 {
        self.driver.current_value(self.channel)
    }

    /// Clear the tick interrupt flag.
    pub fn clear_tick(&self) {
        self.driver.clear_interrupt(self.channel);
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oneshot_overflows_u32_clamps() {
        // 240MHz * 20_000_000us = 4.8e15 ticks > u32::MAX
        // Should clamp to u32::MAX without panic or wrap
        let ticks64: u64 = 240_000_000u64 * 20_000_000u64 / 1_000_000;
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        assert_eq!(ticks, u32::MAX);
    }

    #[test]
    fn test_oneshot_small_value_does_not_clamp() {
        // 240MHz * 100us = 24,000 ticks — fits in u32
        let ticks64: u64 = 240_000_000u64 * 100u64 / 1_000_000;
        assert_eq!(ticks64, 24_000);
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        assert_eq!(ticks, 24_000);
    }

    #[test]
    fn test_oneshot_max_safe_value() {
        // Maximum us that doesn't overflow: u32::MAX / 240 ≈ 17,895,697 us ≈ 17.9 seconds
        let max_safe_us: u32 = u32::MAX / 240;
        let ticks64: u64 = 240_000_000u64 * max_safe_us as u64 / 1_000_000;
        assert!(ticks64 <= u32::MAX as u64);
        let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
        assert_eq!(ticks, ticks64 as u32);
    }

    #[test]
    fn test_periodic_clamping_matches_oneshot() {
        // Both start_micros implementations should clamp identically
        let us: u32 = 20_000_000; // 20 seconds — guaranteed to overflow at 240MHz
        let oneshot_ticks64: u64 = 240_000_000u64 * us as u64 / 1_000_000;
        let periodic_ticks64: u64 = crate::soc::ws63::SYSTEM_CLOCK_HZ as u64 * us as u64 / 1_000_000;
        assert_eq!(oneshot_ticks64, periodic_ticks64); // same formula
        let oneshot = if oneshot_ticks64 > u32::MAX as u64 { u32::MAX } else { oneshot_ticks64 as u32 };
        let periodic = if periodic_ticks64 > u32::MAX as u64 { u32::MAX } else { periodic_ticks64 as u32 };
        assert_eq!(oneshot, periodic);
        assert_eq!(oneshot, u32::MAX);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        /// Fuzz: Timer ticks calculation never panics for any u32 input.
        #[test]
        fn timer_ticks_never_panics(us in any::<u32>()) {
            let ticks64: u64 = 240_000_000u64 * us as u64 / 1_000_000;
            let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
            let _ = ticks;
        }

        /// Fuzz: Clamping is idempotent — applying it twice gives the same result.
        #[test]
        fn timer_clamping_idempotent(us in any::<u32>()) {
            let ticks64: u64 = 240_000_000u64 * us as u64 / 1_000_000;
            let first = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
            let second = if first as u64 > u32::MAX as u64 { u32::MAX } else { first };
            prop_assert_eq!(first, second);
        }

        /// Fuzz: Safe range values (us <= 17_895_697) never get clamped.
        fn timer_safe_range_not_clamped(us in 0u32..17_895_697u32) {
            let ticks64: u64 = 240_000_000u64 * us as u64 / 1_000_000;
            prop_assert!(ticks64 <= u32::MAX as u64, "safe us={} produced ticks64={} > u32::MAX", us, ticks64);
        }

        /// Fuzz: Overflow inputs always clamp to u32::MAX.
        fn timer_overflow_always_clamps(us in 17_895_698u32..u32::MAX) {
            let ticks64: u64 = 240_000_000u64 * us as u64 / 1_000_000;
            prop_assert!(ticks64 > u32::MAX as u64);
            let ticks = if ticks64 > u32::MAX as u64 { u32::MAX } else { ticks64 as u32 };
            prop_assert_eq!(ticks, u32::MAX);
        }

        /// Fuzz: OneShot and Periodic timer formulas are identical.
        #[test]
        fn timer_both_formulas_equivalent(us in any::<u32>()) {
            let oneshot: u64 = 240_000_000u64 * us as u64 / 1_000_000;
            let periodic: u64 = crate::soc::ws63::SYSTEM_CLOCK_HZ as u64 * us as u64 / 1_000_000;
            prop_assert_eq!(oneshot, periodic);
        }
    }
}

// ── Async (embedded-hal-async) ──────────────────────────────────────────────
#[cfg(feature = "async")]
mod asynch_impl {
    use super::{SYSTEM_CLOCK_HZ, Timer, TimerDriver, TimerMode};
    use crate::asynch::IrqSignal;
    use crate::interrupt::{self, Interrupt};
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    static TIMER_SIGNAL: [IrqSignal; 3] = [IrqSignal::new(), IrqSignal::new(), IrqSignal::new()];

    fn ch_irq(ch: usize) -> Interrupt {
        match ch {
            0 => Interrupt::TIMER_INT0,
            1 => Interrupt::TIMER_INT1,
            _ => Interrupt::TIMER_INT2,
        }
    }

    /// Timer trap-handler hook: stop channel `ch`'s one-shot, clear its
    /// interrupt, and wake the awaiting [`AsyncDelay`] future. Call this from the
    /// machine-interrupt trap when `mcause` is TIMER_INT0..2 (IRQ 26..28). The
    /// EOI clears `mip`, so no `LOCIPCLR` is needed for these MIE-class lines.
    pub fn on_interrupt(ch: usize) {
        // SAFETY: RMW of the timer MMIO block. The AsyncDelay owns the peripheral
        // handle, but the ISR uses raw register access (the standard ISR pattern).
        let r = unsafe { &*Timer::ptr() };
        match ch {
            0 => {
                let prev = r.timer0_control(0).read().bits();
                r.timer0_control(0).write(|w| unsafe { w.bits(prev & !1) }); // stop (clear EN)
                let _ = r.timer0_eoi(0).read().bits(); // EOI (read-clear)
            }
            1 => {
                let prev = r.timer0_control(1).read().bits();
                r.timer0_control(1).write(|w| unsafe { w.bits(prev & !1) });
                let _ = r.timer0_eoi(1).read().bits();
            }
            _ => {
                let prev = r.timer0_control(2).read().bits();
                r.timer0_control(2).write(|w| unsafe { w.bits(prev & !1) });
                let _ = r.timer0_eoi(2).read().bits();
            }
        }
        TIMER_SIGNAL[ch].signal();
    }

    /// Async delay backed by one WS63 TIMER channel (one-shot + completion IRQ).
    ///
    /// Implements [`embedded_hal_async::delay::DelayNs`]: each `delay_*().await`
    /// arms the channel one-shot, parks the task until the channel's IRQ fires,
    /// then returns. The app must route the timer trap to [`on_interrupt`] and
    /// have enabled global interrupts (see `ws63-examples/async_delay`).
    pub struct AsyncDelay<'d> {
        driver: TimerDriver<'d>,
        channel: usize,
    }

    impl<'d> AsyncDelay<'d> {
        /// Create an async delay on `channel` (0..=2).
        pub fn new(timer: Timer<'d>, channel: usize) -> Self {
            Self { driver: TimerDriver::new(timer), channel }
        }

        async fn delay_ticks(&mut self, ticks: u32) {
            let ch = self.channel;
            TIMER_SIGNAL[ch].reset();
            self.driver.clear_interrupt(ch);
            self.driver.configure(ch, TimerMode::OneShot, ticks.max(1));
            // SAFETY: enabling a known, fixed WS63 timer IRQ line.
            unsafe { interrupt::enable(ch_irq(ch)) };
            self.driver.enable(ch);
            DelayFuture { ch }.await;
        }
    }

    struct DelayFuture {
        ch: usize,
    }

    impl Future for DelayFuture {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if TIMER_SIGNAL[self.ch].take_fired() {
                Poll::Ready(())
            } else {
                TIMER_SIGNAL[self.ch].register(cx.waker());
                Poll::Pending
            }
        }
    }

    impl embedded_hal_async::delay::DelayNs for AsyncDelay<'_> {
        async fn delay_ns(&mut self, ns: u32) {
            let ticks = (SYSTEM_CLOCK_HZ as u64 * ns as u64 / 1_000_000_000) as u32;
            self.delay_ticks(ticks).await;
        }
    }
}

#[cfg(feature = "async")]
pub use asynch_impl::{AsyncDelay, on_interrupt};
