use crate::{hal, pac};

/// Clock-gate enable (clock.rs). UART0's gate is `CKEN_CTL1` bit 18.
#[cfg(feature = "chip-ws63")]
pub(crate) fn clock_gate_uart0_enabled() {
    use hal::clock::Peripheral;

    let (reg_idx, bit) = Peripheral::Uart0.cken_info().expect("UART0 should be a gated peripheral");
    assert_eq!((reg_idx, bit), (1, 18), "UART0 CKEN gate moved");

    // SAFETY: read-only / RMW-set of the clock-enable register.
    let crg = unsafe { &*pac::CldoCrg::PTR };
    let before = crg.cken_ctl1().read().bits();
    assert_ne!(before & (1 << bit), 0, "UART0 clock gate (CKEN_CTL1 bit 18) not set out of reset");

    crg.cken_ctl1().modify(|r, w| unsafe { w.bits(r.bits() | (1 << bit)) });
    let after = crg.cken_ctl1().read().bits();
    assert_ne!(after & (1 << bit), 0, "UART0 clock gate not high after re-enable");
}
