//! BS2X RTC — real-time-clock / timer (IP v150).
//!
//! BS2X-only (`chip-bs21`) implementation of the `rtc` module (WS63 has the older
//! v100 RTC, `rtc.rs`). The v150 RTC is a 64-bit down-counter with a coherent-read
//! handshake. Register map from fbb_bs2x `hal_rtc_v150` (sole ground truth); this
//! drives the RTC0 instance (bs2x-pac `Rtc` @ 0x5702_4100).
//!
//! Reading the 64-bit counter requires the cnt_req → cnt_lock latch handshake
//! (so the two 32-bit halves are coherent).

use crate::peripherals::Rtc as RtcPeriph;
use core::marker::PhantomData;

/// Counter run mode (`control.mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    OneShot = 0,
    Periodic = 1,
    FreeRun = 3,
}

const POLL_LIMIT: u32 = 0xFFFF;

pub struct Rtc<'d> {
    _rtc: PhantomData<RtcPeriph<'d>>,
}

impl<'d> Rtc<'d> {
    fn regs(&self) -> &'static crate::soc::pac::rtc::RegisterBlock {
        // SAFETY: static physical MMIO base (0x5702_4100, RTC0) from bs2x-pac.
        unsafe { &*RtcPeriph::ptr() }
    }

    /// Start the RTC counting from `load` in `mode` (`hal_rtc_v150_config_load` +
    /// `_start`). For a free-running counter use `Mode::FreeRun`.
    pub fn new(_rtc: RtcPeriph<'d>, load: u32, mode: Mode) -> Self {
        let this = Self { _rtc: PhantomData };
        let r = this.regs();
        unsafe {
            r.control().modify(|_, w| w.enable().clear_bit()); // stop before load
            r.control().modify(|_, w| w.mode().bits(mode as u8));
            r.load_count0().write(|w| w.load_count0().bits(load));
            r.control().modify(|_, w| w.enable().set_bit());
        }
        this
    }

    /// Read the live 64-bit counter using the cnt_req → cnt_lock coherent-read
    /// handshake (`hal_rtc_v150_get_current_value`).
    pub fn read_count(&self) -> u64 {
        let r = self.regs();
        if r.control().read().enable().bit_is_clear() {
            return 0;
        }
        // Request a latched snapshot, then wait for the lock to be granted.
        r.control().modify(|_, w| w.cnt_req().set_bit());
        for _ in 0..POLL_LIMIT {
            if r.control().read().cnt_lock().bit_is_set() {
                break;
            }
            core::hint::spin_loop();
        }
        let lo = r.current_value0().read().current_value0().bits();
        let hi = r.current_value1().read().current_value1().bits();
        ((hi as u64) << 32) | lo as u64
    }

    /// True if the timer interrupt is pending (`raw_intr` bit0).
    pub fn is_pending(&self) -> bool {
        self.regs().raw_intr().read().int_status().bit_is_set()
    }

    /// Clear the timer interrupt by reading the per-instance EOI (`eoi_ren` is
    /// read-to-clear).
    pub fn clear_interrupt(&self) {
        let _ = self.regs().eoi_ren().read().bits();
    }
}
