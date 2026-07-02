/// RTC free-running counter advances (rtc.rs) — opt-in (`hil-rtc` feature).
#[cfg(all(feature = "chip-ws63", feature = "hil-rtc"))]
pub(crate) fn rtc_counter_advances() {
    use crate::hal;
    use core::hint::black_box;
    use hal::rtc::{RtcDriver, RtcMode};

    // SAFETY: sequential single-hart run; RTC singleton not otherwise held.
    let mut rtc = RtcDriver::new(unsafe { hal::peripherals::Rtc::steal() });
    rtc.configure(RtcMode::FreeRunning, 0);
    rtc.enable();
    let a = rtc.current_value();
    for _ in 0..3_000_000u32 {
        black_box(0u32);
    }
    let b = rtc.current_value();
    rtc.disable();
    assert_ne!(a, b, "RTC counter did not advance: a=0x{:08x} b=0x{:08x}", a, b);
}
