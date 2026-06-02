//! Async support for the WS63 HAL (`async` feature).
//!
//! Two pieces shared by the async drivers:
//!
//! * [`block_on`] — a minimal single-future executor. It polls the future and,
//!   while it is `Pending`, sleeps the core with `wfi`. The hardware interrupt
//!   that will eventually complete the future wakes the core, and `block_on`
//!   re-polls. No allocator, no heap, no global executor — enough to drive one
//!   `embedded-hal-async` future to completion. (For multitasking, run these
//!   futures under a real executor such as Embassy instead.)
//!
//! * [`IrqSignal`] — the bridge from an interrupt handler to a waiting future:
//!   the driver's ISR calls [`IrqSignal::signal`]; the future polls
//!   [`IrqSignal::take_fired`] and parks its waker via [`IrqSignal::register`].
//!   It is `const`-constructible so drivers can hold one in a `static`.
//!
//! The async drivers (see [`crate::timer::AsyncDelay`], the `Wait` impl in
//! [`crate::gpio`]) do NOT install interrupt handlers themselves — the
//! application routes its trap to the driver's `on_interrupt` (mirroring the
//! `timer_irq` / `gpio_irq` examples). So enabling the `async` feature never
//! changes the behaviour of firmware that doesn't use it.

use core::cell::Cell;
use core::future::Future;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use critical_section::Mutex;
use portable_atomic::{AtomicBool, Ordering};

// A no-op waker: `block_on` re-polls after every `wfi`, so the waker itself does
// not need to do anything — the `IrqSignal.fired` flag carries the real signal.
static VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW_WAKER, |_| {}, |_| {}, |_| {});
const RAW_WAKER: RawWaker = RawWaker::new(core::ptr::null(), &VTABLE);

#[inline]
fn wait_for_interrupt() {
    #[cfg(target_arch = "riscv32")]
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }
    #[cfg(not(target_arch = "riscv32"))]
    core::hint::spin_loop();
}

/// Drive a single future to completion, sleeping (`wfi`) between polls.
///
/// Global interrupts must be enabled (`interrupt::enable_global`) and the
/// completing peripheral's IRQ routed to its `on_interrupt`, or the `wfi` will
/// never wake.
pub fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = core::pin::pin!(fut);
    // SAFETY: the vtable's clone returns the same null-data RawWaker; wake/drop
    // are no-ops. Nothing dereferences the (null) data pointer.
    let waker = unsafe { Waker::from_raw(RAW_WAKER) };
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        wait_for_interrupt();
    }
}

/// One-shot interrupt→future signal: an ISR calls [`signal`](Self::signal), the
/// future polls [`take_fired`](Self::take_fired) and parks via
/// [`register`](Self::register). `const`-constructible for use in a `static`.
pub struct IrqSignal {
    fired: AtomicBool,
    waker: Mutex<Cell<Option<Waker>>>,
}

impl IrqSignal {
    /// Create a cleared signal.
    pub const fn new() -> Self {
        Self { fired: AtomicBool::new(false), waker: Mutex::new(Cell::new(None)) }
    }

    /// ISR side: mark the event fired and wake any parked waker.
    pub fn signal(&self) {
        self.fired.store(true, Ordering::Release);
        critical_section::with(|cs| {
            if let Some(w) = self.waker.borrow(cs).take() {
                w.wake();
            }
        });
    }

    /// Future side: park `waker` to be woken when the event fires.
    pub fn register(&self, waker: &Waker) {
        critical_section::with(|cs| self.waker.borrow(cs).set(Some(waker.clone())));
    }

    /// Future side: consume the fired flag (`true` once per [`signal`](Self::signal)).
    pub fn take_fired(&self) -> bool {
        self.fired.swap(false, Ordering::Acquire)
    }

    /// Clear both the fired flag and any parked waker (call before arming).
    pub fn reset(&self) {
        self.fired.store(false, Ordering::Relaxed);
        critical_section::with(|cs| self.waker.borrow(cs).set(None));
    }
}

impl Default for IrqSignal {
    fn default() -> Self {
        Self::new()
    }
}
