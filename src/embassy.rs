//! embassy-time `Driver` for the WS63 (`embassy` feature).
//!
//! Makes WS63 an [embassy-time](https://docs.rs/embassy-time) provider so
//! `Timer::after`, `Instant`, `Ticker`, etc. work under embassy-executor:
//!
//! * `now()` reads the **TCXO 64-bit free-running counter** (24 MHz) and scales
//!   it to embassy-time's 1 MHz tick (microseconds). It is monotonic and tracks
//!   real (virtual, on QEMU) elapsed time.
//! * Alarms use **one TIMER channel** (`ALARM_CH`) as a one-shot; on expiry the
//!   app routes the trap to [`on_alarm_interrupt`], which drains the expired
//!   wakers and re-arms for the next deadline ([`embassy_time_queue_utils`]).
//!
//! Requirements (see `ws63-examples/embassy_multitask`):
//! * The application enables `embassy-time/tick-hz-1_000_000` (matches `TICK_HZ`).
//! * The application routes the TIMER alarm-channel trap (mcause TIMER_INT0) to
//!   [`on_alarm_interrupt`] and has enabled global interrupts.

use core::cell::RefCell;
use core::task::Waker;

use critical_section::{CriticalSection, Mutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;

use crate::interrupt::{self, Interrupt};
use crate::peripherals::{Tcxo, Timer};
use crate::soc::ws63::SYSTEM_CLOCK_HZ;

const TCXO_HZ: u64 = 24_000_000;
/// embassy-time tick rate — MUST match the app's `embassy-time/tick-hz-*` feature.
const TICK_HZ: u64 = 1_000_000;
/// TIMER channel reserved for embassy-time alarms (its IRQ is TIMER_INT0 = 26).
const ALARM_CH: usize = 0;

/// Read the TCXO 64-bit counter and scale to embassy-time ticks (µs).
fn now_ticks() -> u64 {
    // SAFETY: read-only/RMW of the TCXO MMIO block via the singleton pointer.
    let r = unsafe { &*Tcxo::ptr() };
    let st = r.tcxo_status().read().bits();
    unsafe { r.tcxo_status().write(|w| w.bits(st | 0x01)) }; // refresh-latch
    for _ in 0..100 {
        if r.tcxo_status().read().bits() & 0x10 != 0 {
            break; // count-valid
        }
    }
    let c0 = r.tcxo_count0().read().bits() as u64;
    let c1 = r.tcxo_count1().read().bits() as u64;
    let c2 = r.tcxo_count2().read().bits() as u64;
    let c3 = r.tcxo_count3().read().bits() as u64;
    let count = (c3 << 48) | (c2 << 32) | (c1 << 16) | c0;
    // 24 MHz counter -> 1 MHz ticks.
    count * TICK_HZ / TCXO_HZ
}

struct Ws63Driver {
    queue: Mutex<RefCell<Queue>>,
}

impl Ws63Driver {
    /// Program the alarm TIMER one-shot to fire at tick `at` (or disable it for
    /// `u64::MAX`). Called inside a critical section.
    fn set_alarm(&self, _cs: CriticalSection<'_>, at: u64) {
        // SAFETY: RMW of the TIMER MMIO block via the singleton pointer.
        let r = unsafe { &*Timer::ptr() };
        // Stop + clear the alarm channel first.
        let prev = r.timer0_control(ALARM_CH).read().bits();
        unsafe { r.timer0_control(ALARM_CH).write(|w| w.bits(prev & !1)) };
        let _ = r.timer0_eoi(ALARM_CH).read().bits();

        if at == u64::MAX {
            return; // no pending timers
        }

        let now = now_ticks();
        let delta_ticks = at.saturating_sub(now).max(1);
        // ticks(µs) -> TIMER counts at SYSTEM_CLOCK_HZ, clamped to u32.
        let mut counts = delta_ticks * SYSTEM_CLOCK_HZ as u64 / TICK_HZ;
        if counts == 0 {
            counts = 1;
        }
        if counts > u32::MAX as u64 {
            counts = u32::MAX as u64;
        }
        unsafe {
            r.timer0_load_count(ALARM_CH).write(|w| w.bits(counts as u32));
            r.timer0_control(ALARM_CH).write(|w| w.bits(1)); // EN, IRQ unmasked
            interrupt::enable(Interrupt::TIMER_INT0);
        }
    }
}

impl Driver for Ws63Driver {
    fn now(&self) -> u64 {
        now_ticks()
    }

    fn schedule_wake(&self, at: u64, waker: &Waker) {
        critical_section::with(|cs| {
            let mut q = self.queue.borrow(cs).borrow_mut();
            if q.schedule_wake(at, waker) {
                // Earliest deadline changed — re-arm the hardware alarm.
                let next = q.next_expiration(now_ticks());
                self.set_alarm(cs, next);
            }
        });
    }
}

embassy_time_driver::time_driver_impl!(
    static DRIVER: Ws63Driver = Ws63Driver {
        queue: Mutex::new(RefCell::new(Queue::new()))
    }
);

/// Alarm-channel trap hook. Call this from the application's machine-interrupt
/// trap when `mcause` is TIMER_INT0 (the embassy alarm channel). It wakes the
/// expired timers and re-arms the alarm for the next deadline.
pub fn on_alarm_interrupt() {
    critical_section::with(|cs| {
        let mut q = DRIVER.queue.borrow(cs).borrow_mut();
        let next = q.next_expiration(now_ticks());
        DRIVER.set_alarm(cs, next);
    });
}
