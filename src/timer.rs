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
            0 => { let _ = r.timer0_eoi(0).read().bits(); }
            1 => { let _ = r.timer0_eoi(1).read().bits(); }
            2 => { let _ = r.timer0_eoi(2).read().bits(); }
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
    pub fn start_micros(&mut self, us: u32) {
        let ticks = ((SYSTEM_CLOCK_HZ as u64 * us as u64) / 1_000_000) as u32;
        self.start(ticks);
    }

    /// Start the timer for the given duration in milliseconds.
    pub fn start_millis(&mut self, ms: u32) {
        let ticks = ((SYSTEM_CLOCK_HZ as u64 * ms as u64) / 1_000) as u32;
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
        let ticks = ((SYSTEM_CLOCK_HZ as u64 * ns as u64) / 1_000_000_000) as u32;
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
        let ticks = ((SYSTEM_CLOCK_HZ as u64 * us as u64) / 1_000_000) as u32;
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
