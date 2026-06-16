//! embassy-time `Driver` for the HiSilicon WS63 / BS2X family (`embassy` feature).
//!
//! Makes the chip an [embassy-time](https://docs.rs/embassy-time) provider so
//! `Timer::after`, `Instant`, `Ticker`, etc. work under embassy-executor:
//!
//! * `now()` reads the **TCXO 64-bit free-running counter** (the crystal rate
//!   [`crate::soc::chip::TCXO_HZ`] — 24 MHz on WS63, 32 MHz on BS2X) and scales
//!   it to embassy-time's 1 MHz tick (microseconds). It is monotonic and tracks
//!   real (virtual, on QEMU) elapsed time.
//! * Alarms use **one TIMER channel** (`ALARM_CH`) as a one-shot; on expiry the
//!   app routes the trap to [`on_alarm_interrupt`], which drains the expired
//!   wakers and re-arms for the next deadline ([`embassy_time_queue_utils`]).
//!
//! The TCXO and TIMER register blocks are register-identical across the family
//! (verified vs the fbb_ws63 / fbb_bs2x C SDKs); the only per-chip values — the
//! crystal rate and the alarm IRQ — come from `soc::chip`.
//!
//! Requirements (see `examples/ws63/embassy_multitask`, `examples/bs21/embassy_multitask`):
//! * The application enables `embassy-time/tick-hz-1_000_000` (matches `TICK_HZ`).
//! * The application routes the TIMER alarm-channel trap (`mcause & 0xFFF` ==
//!   [`ALARM_IRQ`]) to [`on_alarm_interrupt`] and has enabled global interrupts.

use core::cell::RefCell;
use core::task::Waker;

use critical_section::{CriticalSection, Mutex};
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;

use crate::interrupt;
use crate::peripherals::{Tcxo, Timer};
use crate::soc::chip::TIMER_CLOCK_HZ;

const TCXO_HZ: u64 = crate::soc::chip::TCXO_HZ as u64;
/// embassy-time tick rate — MUST match the app's `embassy-time/tick-hz-*` feature.
const TICK_HZ: u64 = 1_000_000;
/// TIMER channel reserved for embassy-time alarms.
const ALARM_CH: usize = 0;

/// IRQ number of the embassy-time alarm interrupt — **26 on WS63** (`TIMER_INT0`,
/// a standard `mie`-bit local interrupt), **53 on BS2X** (`TIMER_0`, a LOCI custom
/// local interrupt). Both are delivered with `mcause = <irq>`, so the application's
/// machine-trap handler routes the alarm by testing `mcause & 0xFFF == ALARM_IRQ`.
pub const ALARM_IRQ: u32 = crate::soc::chip::ALARM_INTERRUPT as u32;

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
    // TCXO crystal counter -> 1 MHz ticks (TCXO_HZ is per-chip from soc::chip).
    count * TICK_HZ / TCXO_HZ
}

struct HisiTimeDriver {
    queue: Mutex<RefCell<Queue>>,
}

impl HisiTimeDriver {
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
        // ticks(µs) -> TIMER counts at the TIMER clock (TCXO crystal), clamped to u32.
        let mut counts = delta_ticks * TIMER_CLOCK_HZ as u64 / TICK_HZ;
        if counts == 0 {
            counts = 1;
        }
        if counts > u32::MAX as u64 {
            counts = u32::MAX as u64;
        }
        unsafe {
            r.timer0_load_count(ALARM_CH).write(|w| w.bits(counts as u32));
            // Raw control = 0x1: enable (bit0), mode bits = 0b00 (one-shot in HW —
            // do NOT use the PAC's typed `.mode()`, whose enum is inverted vs the
            // dw_apb timer), int_mask (bit3) = 0 = unmasked. Verified vs fbb SDKs.
            r.timer0_control(ALARM_CH).write(|w| w.bits(1));
            interrupt::enable(crate::soc::chip::ALARM_INTERRUPT);
        }
    }
}

impl Driver for HisiTimeDriver {
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
    static DRIVER: HisiTimeDriver = HisiTimeDriver {
        queue: Mutex::new(RefCell::new(Queue::new()))
    }
);

/// Alarm-channel handler: wakes the expired timers and re-arms the alarm for the
/// next deadline. Invoked automatically via the named `TIMER_INT0` device.x handler
/// below (hisi-riscv-rt's direct-mode `__rt_irq_dispatch` routes the alarm IRQ here);
/// also kept `pub` so an app that owns its own trap (direct mtvec) can call it.
pub fn on_alarm_interrupt() {
    critical_section::with(|cs| {
        let mut q = DRIVER.queue.borrow(cs).borrow_mut();
        let next = q.next_expiration(now_ticks());
        DRIVER.set_alarm(cs, next);
    });
}

/// Named hisi-riscv-rt interrupt handler for the embassy-time alarm IRQ — **WS63:
/// `TIMER_INT0` = IRQ 26** (`ALARM_INTERRUPT`). With the rt's direct-mode routing
/// (`__rt_irq_dispatch` indexes `__INTERRUPTS` by IRQ number to the matching
/// `device.x` named symbol), this fires automatically when the alarm TIMER channel
/// expires, so an embassy app needs **no `mcause` trap of its own** — just
/// `embassy-executor` + `Timer::after` (the driver's `set_alarm` already calls
/// `interrupt::enable(ALARM_INTERRUPT)`, which now also raises its `LOCIPRI`, plus
/// `interrupt::enable_global()`). Strong symbol overriding the weak `device.x`
/// PROVIDE; only present with the `embassy` feature, so the HIL/test build (no
/// `embassy`) is free to define its own `TIMER_INT0`. BS2X's alarm is a different
/// IRQ/symbol — added once a BS2X board validates it.
#[cfg(all(feature = "embassy", feature = "chip-ws63"))]
#[unsafe(no_mangle)]
extern "C" fn TIMER_INT0() {
    on_alarm_interrupt();
}
