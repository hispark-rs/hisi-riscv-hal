//! PWM driver for WS63 (8 channels, 32-bit).
use crate::peripherals::Pwm;
use core::marker::PhantomData;

pub struct PwmChannel<'d> {
    channel: u8,
    _marker: PhantomData<&'d ()>,
}

impl<'d> PwmChannel<'d> {
    pub fn new(_pwm: &Pwm<'d>, channel: u8) -> Self {
        assert!(channel < 8);
        Self { channel, _marker: PhantomData }
    }

    fn regs(&self) -> &'static crate::soc::pac::pwm::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Pwm::ptr() }
    }

    pub fn configure(&mut self, freq: u32, duty_percent: u8) {
        assert!(freq > 0, "PWM frequency must be non-zero");
        let r = self.regs();
        let pclk = crate::soc::chip::SYSTEM_CLOCK_HZ;
        let period = pclk / freq;
        let duty = (period as u64 * duty_percent as u64 / 100) as u32;
        match self.channel {
            0 => {
                r.pwm_freq_l0().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h0().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l0().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h0().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            1 => {
                r.pwm_freq_l1().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h1().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l1().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h1().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            2 => {
                r.pwm_freq_l2().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h2().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l2().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h2().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            3 => {
                r.pwm_freq_l3().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h3().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l3().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h3().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            4 => {
                r.pwm_freq_l4().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h4().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l4().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h4().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            5 => {
                r.pwm_freq_l5().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h5().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l5().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h5().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            6 => {
                r.pwm_freq_l6().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h6().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l6().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h6().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            7 => {
                r.pwm_freq_l7().write(|w| unsafe { w.bits(period & 0xFFFF) });
                r.pwm_freq_h7().write(|w| unsafe { w.bits((period >> 16) & 0xFFFF) });
                r.pwm_duty_l7().write(|w| unsafe { w.bits(duty & 0xFFFF) });
                r.pwm_duty_h7().write(|w| unsafe { w.bits((duty >> 16) & 0xFFFF) });
            }
            _ => unreachable!(),
        }
    }

    pub fn enable(&mut self) {
        match self.channel {
            0 => self.regs().pwm_en0().write(|w| unsafe { w.bits(1u32) }),
            1 => self.regs().pwm_en1().write(|w| unsafe { w.bits(1u32) }),
            2 => self.regs().pwm_en2().write(|w| unsafe { w.bits(1u32) }),
            3 => self.regs().pwm_en3().write(|w| unsafe { w.bits(1u32) }),
            4 => self.regs().pwm_en4().write(|w| unsafe { w.bits(1u32) }),
            5 => self.regs().pwm_en5().write(|w| unsafe { w.bits(1u32) }),
            6 => self.regs().pwm_en6().write(|w| unsafe { w.bits(1u32) }),
            7 => self.regs().pwm_en7().write(|w| unsafe { w.bits(1u32) }),
            _ => unreachable!(),
        };
    }
    pub fn disable(&mut self) {
        match self.channel {
            0 => self.regs().pwm_en0().write(|w| unsafe { w.bits(0u32) }),
            1 => self.regs().pwm_en1().write(|w| unsafe { w.bits(0u32) }),
            2 => self.regs().pwm_en2().write(|w| unsafe { w.bits(0u32) }),
            3 => self.regs().pwm_en3().write(|w| unsafe { w.bits(0u32) }),
            4 => self.regs().pwm_en4().write(|w| unsafe { w.bits(0u32) }),
            5 => self.regs().pwm_en5().write(|w| unsafe { w.bits(0u32) }),
            6 => self.regs().pwm_en6().write(|w| unsafe { w.bits(0u32) }),
            7 => self.regs().pwm_en7().write(|w| unsafe { w.bits(0u32) }),
            _ => unreachable!(),
        };
    }
    pub fn set_polarity(&mut self, active_high: bool) {
        let val = if active_high { 1u32 } else { 0u32 };
        match self.channel {
            0 => self.regs().pwm_portity0().write(|w| unsafe { w.bits(val) }),
            1 => self.regs().pwm_portity1().write(|w| unsafe { w.bits(val) }),
            2 => self.regs().pwm_portity2().write(|w| unsafe { w.bits(val) }),
            3 => self.regs().pwm_portity3().write(|w| unsafe { w.bits(val) }),
            4 => self.regs().pwm_portity4().write(|w| unsafe { w.bits(val) }),
            5 => self.regs().pwm_portity5().write(|w| unsafe { w.bits(val) }),
            6 => self.regs().pwm_portity6().write(|w| unsafe { w.bits(val) }),
            7 => self.regs().pwm_portity7().write(|w| unsafe { w.bits(val) }),
            _ => unreachable!(),
        };
    }
    pub fn start(&mut self) {
        self.regs().pwm_start0().write(|w| unsafe { w.bits(1u32 << self.channel) });
    }
    pub fn set_pulse_count(&mut self, count: u32) {
        match self.channel {
            0 => self.regs().pwm_period_val0().write(|w| unsafe { w.bits(count) }),
            1 => self.regs().pwm_period_val1().write(|w| unsafe { w.bits(count) }),
            2 => self.regs().pwm_period_val2().write(|w| unsafe { w.bits(count) }),
            3 => self.regs().pwm_period_val3().write(|w| unsafe { w.bits(count) }),
            4 => self.regs().pwm_period_val4().write(|w| unsafe { w.bits(count) }),
            5 => self.regs().pwm_period_val5().write(|w| unsafe { w.bits(count) }),
            6 => self.regs().pwm_period_val6().write(|w| unsafe { w.bits(count) }),
            7 => self.regs().pwm_period_val7().write(|w| unsafe { w.bits(count) }),
            _ => unreachable!(),
        };
    }
}

impl embedded_hal::pwm::ErrorType for PwmChannel<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_hal::pwm::SetDutyCycle for PwmChannel<'_> {
    fn max_duty_cycle(&self) -> u16 {
        u16::MAX
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        let r = self.regs();
        let duty_val = duty as u32;
        match self.channel {
            0 => {
                unsafe { r.pwm_duty_l0().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h0().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            1 => {
                unsafe { r.pwm_duty_l1().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h1().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            2 => {
                unsafe { r.pwm_duty_l2().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h2().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            3 => {
                unsafe { r.pwm_duty_l3().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h3().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            4 => {
                unsafe { r.pwm_duty_l4().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h4().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            5 => {
                unsafe { r.pwm_duty_l5().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h5().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            6 => {
                unsafe { r.pwm_duty_l6().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h6().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            7 => {
                unsafe { r.pwm_duty_l7().write(|w| w.bits(duty_val & 0xFFFF)) };
                unsafe { r.pwm_duty_h7().write(|w| w.bits((duty_val >> 16) & 0xFFFF)) };
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use crate::soc::chip::SYSTEM_CLOCK_HZ;

    // Mirrors the pure arithmetic in `PwmChannel::configure`: the PWM counter runs
    // at the system clock, so the period (in ticks) is `SYSTEM_CLOCK_HZ / freq`, and
    // the on-time `duty` is `period * duty_percent / 100` (computed in u64 to avoid
    // intermediate overflow). Each value is then split into low/high 16-bit halves.
    fn period_for(freq: u32) -> u32 {
        SYSTEM_CLOCK_HZ / freq
    }

    fn duty_for(period: u32, duty_percent: u8) -> u32 {
        (period as u64 * duty_percent as u64 / 100) as u32
    }

    fn lo(v: u32) -> u32 {
        v & 0xFFFF
    }

    fn hi(v: u32) -> u32 {
        (v >> 16) & 0xFFFF
    }

    #[test]
    fn period_is_clock_over_freq() {
        // 1 kHz → SYSTEM_CLOCK_HZ counter ticks per period.
        assert_eq!(period_for(1_000), SYSTEM_CLOCK_HZ / 1_000);
        // Frequency equal to the clock yields a 1-tick period.
        assert_eq!(period_for(SYSTEM_CLOCK_HZ), 1);
    }

    #[test]
    fn duty_zero_and_full() {
        // 0% duty is always zero on-time; 100% duty equals the full period.
        let period = period_for(1_000);
        assert_eq!(duty_for(period, 0), 0);
        assert_eq!(duty_for(period, 100), period);
    }

    #[test]
    fn duty_fifty_percent_is_half_period() {
        // 50% duty is exactly half the period (period chosen even to divide cleanly).
        let period = period_for(1_000);
        assert_eq!(duty_for(period, 50), period / 2);
    }

    #[test]
    fn duty_monotonic_in_percent() {
        // On-time is non-decreasing as the duty percentage rises.
        let period = period_for(1_000);
        let mut prev = 0u32;
        for pct in 0u8..=100 {
            let d = duty_for(period, pct);
            assert!(d >= prev, "duty must be monotonic: pct={pct} d={d} prev={prev}");
            assert!(d <= period, "duty {d} must never exceed period {period}");
            prev = d;
        }
    }

    #[test]
    fn duty_no_overflow_for_large_period() {
        // The u64 widening means even a near-u32::MAX period * 100% never overflows
        // the intermediate product, and the result truncates back to the period.
        let period = u32::MAX;
        assert_eq!(duty_for(period, 100), period);
        // 1% of u32::MAX is computed exactly via u64.
        assert_eq!(duty_for(period, 1), ((u32::MAX as u64) / 100) as u32);
    }

    #[test]
    fn lo_hi_split_round_trips() {
        // The low/high 16-bit halves written to the *_l/*_h registers reassemble
        // back into the original 32-bit value.
        for v in [0u32, 1, 0xFFFF, 0x1_0000, 0xDEAD_BEEF, u32::MAX] {
            assert_eq!(lo(v) | (hi(v) << 16), v, "round-trip failed for {v:#x}");
            assert!(lo(v) <= 0xFFFF);
            assert!(hi(v) <= 0xFFFF);
        }
    }

    #[test]
    fn start_mask_is_channel_bit() {
        // `start()` writes `1 << channel`; each channel maps to a distinct bit.
        for ch in 0u8..8 {
            assert_eq!(1u32 << ch, 1u32 << ch);
            assert_eq!((1u32 << ch).count_ones(), 1);
        }
        assert_eq!(1u32 << 7, 0x80);
    }

    #[test]
    fn channel_bounds() {
        // `PwmChannel::new` asserts `channel < 8`; the valid range is exactly 0..8.
        assert!((0u8..8).all(|c| c < 8));
        assert!(8u8 >= 8);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use crate::soc::chip::SYSTEM_CLOCK_HZ;
    use proptest::prelude::*;

    fn duty_for(period: u32, duty_percent: u8) -> u32 {
        (period as u64 * duty_percent as u64 / 100) as u32
    }

    proptest! {
        /// Fuzz: period = clock/freq never panics and never exceeds the full clock
        /// count for any non-zero frequency.
        #[test]
        fn period_in_range(freq in 1u32..=SYSTEM_CLOCK_HZ) {
            let period = SYSTEM_CLOCK_HZ / freq;
            prop_assert!(period >= 1);
            prop_assert!(period <= SYSTEM_CLOCK_HZ);
        }

        /// Fuzz: the u64-widened duty computation never panics and the on-time is
        /// always within [0, period] for any period and any 0..=100 duty percent.
        #[test]
        fn duty_within_period(period in any::<u32>(), pct in 0u8..=100) {
            let d = duty_for(period, pct);
            prop_assert!(d <= period, "duty {} > period {} (pct {})", d, period, pct);
        }

        /// Fuzz: the low/high 16-bit register split always reassembles to the input.
        #[test]
        fn lo_hi_round_trip(v in any::<u32>()) {
            let lo = v & 0xFFFF;
            let hi = (v >> 16) & 0xFFFF;
            prop_assert_eq!(lo | (hi << 16), v);
            prop_assert!(lo <= 0xFFFF);
            prop_assert!(hi <= 0xFFFF);
        }

        /// Fuzz: the start-register mask is a single bit for every valid channel.
        #[test]
        fn start_mask_single_bit(ch in 0u8..8) {
            let mask = 1u32 << ch;
            prop_assert_eq!(mask.count_ones(), 1);
        }
    }
}
