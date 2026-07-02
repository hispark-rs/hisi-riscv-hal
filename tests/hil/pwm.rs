use crate::{hal, pac};

/// PWM typed config + clock-tree bring-up (pwm.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn pwm_configure_and_enable() {
    use hal::pwm::{Duty, PwmChannel, PwmChannelId, PwmPeriod};

    // SAFETY: sequential single-hart run; PWM singleton not otherwise held.
    let pwm = unsafe { hal::peripherals::Pwm::steal() };
    let mut ch = PwmChannel::new(&pwm, PwmChannelId::Ch0);
    let period = PwmPeriod::from_count(24_000).expect("non-zero period");
    ch.configure(period, Duty::HALF);
    ch.enable();

    // SAFETY: read-only MMIO loads of the PWM ch0 registers.
    let r = unsafe { &*pac::Pwm::PTR };
    assert_eq!(r.pwm_freq_l0().read().bits(), 24_000, "PWM freq_l0 not latched");
    assert_eq!(r.pwm_freq_h0().read().bits(), 0, "PWM freq_h0 unexpectedly non-zero");
    assert_eq!(r.pwm_duty_l0().read().bits(), 12_000, "PWM 50% duty not latched");
    assert_ne!(r.pwm_en0().read().bits() & 1, 0, "PWM ch0 not enabled");

    ch.disable();
    assert_eq!(r.pwm_en0().read().bits() & 1, 0, "PWM ch0 still enabled after disable");
}

/// PWM embedded-hal duty validation (pwm.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn pwm_set_duty_cycle_rejects_out_of_range() {
    use embedded_hal::pwm::SetDutyCycle;
    use hal::pwm::{Duty, PwmChannel, PwmChannelId, PwmError, PwmPeriod};

    // SAFETY: sequential single-hart run; PWM singleton not otherwise held.
    let pwm = unsafe { hal::peripherals::Pwm::steal() };
    let mut ch = PwmChannel::new(&pwm, PwmChannelId::Ch0);
    let period = PwmPeriod::from_count(100).unwrap();
    ch.configure(period, Duty::HALF);

    assert_eq!(ch.max_duty_cycle(), 100);
    ch.set_duty_cycle(75).expect("in-range duty should be accepted");
    assert!(matches!(ch.set_duty_cycle(101), Err(PwmError::DutyOutOfRange)));

    // SAFETY: read-only MMIO load of the PWM ch0 duty register.
    let r = unsafe { &*pac::Pwm::PTR };
    assert_eq!(r.pwm_duty_l0().read().bits(), 75, "rejected duty should not overwrite last valid duty");
}
