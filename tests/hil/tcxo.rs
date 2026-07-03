use crate::{hal, pac};
use core::hint::black_box;

/// Read a real SoC register through the PAC singleton handed over by `#[init]`
/// and assert a structural fact about it. Reads only — no writes, no state change.
pub(crate) fn read_tcxo_status_register(p: pac::Peripherals) {
    // `bits()` performs a volatile 32-bit load from 0x4400_04c0 + offset.
    let status = p.tcxo.tcxo_status().read().bits();
    assert_ne!(status, 0xFFFF_FFFF, "TCXO status read returned the bus-floating all-ones pattern");
}

/// TCXO free-running counter is monotonic (tcxo.rs). Read the 32-bit counter twice
/// with a busy-wait between; assert it strictly increased.
pub(crate) fn tcxo_counter_monotonic() {
    use hal::tcxo::TcxoDriver;

    // SAFETY: sequential single-hart run; TCXO singleton not otherwise held.
    let tcxo = TcxoDriver::new(unsafe { hal::peripherals::Tcxo::steal() });

    let a = tcxo.read_counter32().expect("first TCXO refresh timed out");
    for _ in 0..50_000 {
        black_box(0u32);
    }
    let b = tcxo.read_counter32().expect("second TCXO refresh timed out");
    assert!(b > a, "TCXO counter did not advance: first=0x{:08x} second=0x{:08x}", a, b);
}

/// TCXO full 64-bit counter path (tcxo.rs). Exercises the four-register 64-bit
/// assembly on live silicon and proves it also refreshes/advances.
pub(crate) fn tcxo_counter64_monotonic() {
    use hal::tcxo::TcxoDriver;

    // SAFETY: sequential single-hart run; TCXO singleton not otherwise held.
    let tcxo = TcxoDriver::new(unsafe { hal::peripherals::Tcxo::steal() });

    let a = tcxo.read_counter().expect("first TCXO64 refresh timed out");
    for _ in 0..50_000 {
        black_box(0u32);
    }
    let b = tcxo.read_counter().expect("second TCXO64 refresh timed out");
    assert!(b > a, "TCXO64 counter did not advance: first=0x{a:016x} second=0x{b:016x}");
}
