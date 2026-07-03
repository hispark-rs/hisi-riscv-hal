use crate::hal;
use core::hint::black_box;

/// Timer counter advances (timer.rs / examples/ws63/timer_irq).
pub(crate) fn timer_counter_advances() {
    use hal::timer::{TimerChannel, TimerDriver, TimerMode};

    // SAFETY: sequential single-hart run; TIMER singleton not otherwise held.
    let timer = TimerDriver::new(unsafe { hal::peripherals::Timer::steal() });
    timer.configure(TimerChannel::Channel0, TimerMode::Periodic, 0x00FF_FFFF);
    timer.enable(TimerChannel::Channel0);

    let a = timer.current_value(TimerChannel::Channel0);
    for _ in 0..50_000 {
        black_box(0u32);
    }
    let b = timer.current_value(TimerChannel::Channel0);
    timer.disable(TimerChannel::Channel0);
    assert_ne!(a, b, "TIMER ch0 current_value did not advance: a=0x{:08x} b=0x{:08x}", a, b);
}
