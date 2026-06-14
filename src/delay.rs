//! Blocking delay driver for WS63.
//!
//! Uses busy-loop timing based on the system clock frequency.
//! Implements `embedded_hal::delay::DelayNs`.

use crate::soc::chip::SYSTEM_CLOCK_HZ;

/// Delay driver using busy-wait loops.
pub struct Delay;

impl Delay {
    /// Create a new delay driver.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Delay
    }

    /// Busy-wait for the given number of microseconds.
    pub fn delay_micros(&self, us: u32) {
        // Each loop iteration is roughly 3 cycles (decrement, compare, branch)
        let cycles_per_loop: u32 = 3;
        let cycles = (SYSTEM_CLOCK_HZ as u64 * us as u64 / 1_000_000) as u32;
        let loops = cycles / cycles_per_loop;
        for _ in 0..loops {
            core::hint::spin_loop();
        }
    }

    /// Busy-wait for the given number of milliseconds.
    pub fn delay_millis(&self, ms: u32) {
        for _ in 0..ms {
            self.delay_micros(1_000);
        }
    }
}

impl embedded_hal::delay::DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        // Use microsecond granularity, rounding up
        let us = ns.div_ceil(1_000);
        self.delay_micros(us);
    }

    fn delay_us(&mut self, us: u32) {
        self.delay_micros(us);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.delay_millis(ms);
    }
}
