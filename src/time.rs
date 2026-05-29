//! Timekeeping types for WS63 HAL.
//!
//! Provides `Instant`, `Duration`, and `Rate` types for timing operations.
//! Uses the TCXO 64-bit counter as the hardware time source.

use crate::peripherals::Tcxo;

/// A measurement of a monotonically non-decreasing clock.
/// Opaque and useful with `Duration`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);

impl Instant {
    /// Returns an instant corresponding to "now".
    pub fn now() -> Self {
        let tcxo = unsafe { &*Tcxo::ptr() };
        // Latch the counter
        let status = tcxo.tcxo_status().read().bits();
        unsafe { tcxo.tcxo_status().write(|w| w.bits(status | 0x01)); }
        while tcxo.tcxo_status().read().bits() & 0x10 == 0 {}
        // Read 64-bit counter
        let c0 = tcxo.tcxo_count0().read().bits() as u64;
        let c1 = tcxo.tcxo_count1().read().bits() as u64;
        let c2 = tcxo.tcxo_count2().read().bits() as u64;
        let c3 = tcxo.tcxo_count3().read().bits() as u64;
        Instant((c3 << 48) | (c2 << 32) | (c1 << 16) | c0)
    }

    /// Returns the amount of time elapsed since this instant.
    pub fn elapsed(&self) -> Duration {
        Instant::now().checked_duration_since(*self).unwrap_or(Duration::from_micros(0))
    }

    /// Returns the amount of time elapsed from another instant to this one.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or(Duration::from_micros(0))
    }

    /// Returns `Some(Duration)` if `earlier` is earlier than `self`, or `None` otherwise.
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        if self.0 >= earlier.0 {
            Some(Duration::from_micros(self.0 - earlier.0))
        } else {
            None
        }
    }

    /// Returns the raw counter value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// A duration of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration(u64); // microseconds

impl Duration {
    /// Create a duration from microseconds.
    pub const fn from_micros(micros: u64) -> Self {
        Duration(micros)
    }

    /// Create a duration from milliseconds.
    pub const fn from_millis(millis: u64) -> Self {
        Duration(millis * 1_000)
    }

    /// Create a duration from seconds.
    pub const fn from_secs(secs: u64) -> Self {
        Duration(secs * 1_000_000)
    }

    /// Return the total number of microseconds.
    pub const fn as_micros(&self) -> u64 {
        self.0
    }

    /// Return the total number of milliseconds.
    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000
    }

    /// Return the total number of seconds.
    pub const fn as_secs(&self) -> u64 {
        self.0 / 1_000_000
    }
}

impl core::ops::Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Duration(self.0 + rhs.0) }
}

impl core::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
}

impl core::ops::Sub for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Duration(self.0.saturating_sub(rhs.0)) }
}

/// A rate in Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate(u32);

impl Rate {
    pub const fn from_hz(hz: u32) -> Self { Rate(hz) }
    pub const fn from_khz(khz: u32) -> Self { Rate(khz * 1_000) }
    pub const fn from_mhz(mhz: u32) -> Self { Rate(mhz * 1_000_000) }
    pub const fn to_hz(&self) -> u32 { self.0 }
    pub const fn to_khz(&self) -> u32 { self.0 / 1_000 }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_constructors() {
        assert_eq!(Duration::from_micros(500).as_micros(), 500);
        assert_eq!(Duration::from_millis(1).as_micros(), 1000);
        assert_eq!(Duration::from_secs(1).as_micros(), 1_000_000);
        assert_eq!(Duration::from_micros(1500).as_millis(), 1);
    }

    #[test]
    fn test_duration_add_sub() {
        let a = Duration::from_micros(100);
        let b = Duration::from_micros(50);
        assert_eq!((a + b).as_micros(), 150);
        assert_eq!((a - b).as_micros(), 50);
        // Saturating subtract
        assert_eq!((b - a).as_micros(), 0);
    }

    #[test]
    fn test_duration_default() {
        assert_eq!(Duration::default().as_micros(), 0);
    }

    #[test]
    fn test_rate_constructors() {
        assert_eq!(Rate::from_hz(1000).to_hz(), 1000);
        assert_eq!(Rate::from_khz(1).to_hz(), 1000);
        assert_eq!(Rate::from_mhz(1).to_hz(), 1_000_000);
        assert_eq!(Rate::from_mhz(240).to_khz(), 240_000);
    }

    #[test]
    fn test_instant_checked_duration() {
        let early = Instant(100);
        let late = Instant(200);
        assert_eq!(late.checked_duration_since(early).unwrap().as_micros(), 100);
        assert!(early.checked_duration_since(late).is_none());
    }
}
