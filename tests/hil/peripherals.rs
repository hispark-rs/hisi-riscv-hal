use crate::hal;

/// HAL `Peripherals` construction smoke test (PAC/HAL structural #8).
#[cfg(feature = "chip-ws63")]
pub(crate) fn hal_peripherals_construct() {
    // SAFETY: sequential single-hart test run; no other live handles.
    let hp = unsafe { hal::Peripherals::steal() };

    assert_eq!(hal::peripherals::Gpio0::ptr() as usize, 0x4402_8000, "HAL GPIO0 ptr mismatch");
    assert_eq!(hal::peripherals::Tcxo::ptr() as usize, 0x4400_04c0, "HAL TCXO ptr mismatch");
    assert_eq!(hal::peripherals::Timer::ptr() as usize, 0x4400_2000, "HAL TIMER ptr mismatch");
    assert_eq!(hal::peripherals::Dma::ptr() as usize, 0x4a00_0000, "HAL DMA ptr mismatch");
    assert_eq!(hal::peripherals::Uart0::ptr() as usize, 0x4401_0000, "HAL UART0 ptr mismatch");

    let _ = hp.GPIO0;
}
