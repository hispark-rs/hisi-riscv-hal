use crate::{hal, pac};

/// UART divider register configuration (uart.rs), default 160 MHz PLL base.
pub(crate) fn uart0_divider_config() {
    use hal::uart::{Config, Uart};

    let cfg = Config::default();
    // SAFETY: sequential single-hart run; UART0 singleton not otherwise held.
    let _uart = Uart::new_uart0(unsafe { hal::peripherals::Uart0::steal() }, cfg);

    let pclk = hal::soc::chip::UART_CLOCK_HZ;
    let div64 = ((pclk as u64) * 4 / (cfg.baudrate.baud() as u64)) as u32;
    let div = div64 >> 6;
    let exp_div_fra = (div64 & 0x3F) as u16;
    let exp_div_l = (div & 0xFF) as u16;
    let exp_div_h = ((div >> 8) & 0xFF) as u16;

    // SAFETY: read-only MMIO loads of the UART0 divider registers.
    let r = unsafe { &*pac::Uart0::PTR };
    assert_eq!(r.div_l().read().bits(), exp_div_l, "UART0 div_l mismatch");
    assert_eq!(r.div_h().read().bits(), exp_div_h, "UART0 div_h mismatch");
    assert_eq!(r.div_fra().read().bits(), exp_div_fra, "UART0 div_fra mismatch");
}

/// UART boot-clock divider configuration (uart.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn uart0_boot_clock_divider_config() {
    use hal::uart::{Config, Uart, UartClock};

    let cfg = Config { clock: UartClock::Boot, ..Config::default() };
    // SAFETY: sequential single-hart run; UART0 singleton not otherwise held.
    let _uart = Uart::new_uart0(unsafe { hal::peripherals::Uart0::steal() }, cfg);

    let pclk = hal::soc::chip::uart_boot_clock_hz();
    assert!(
        pclk == hal::soc::chip::UART_BOOT_CLOCK_24M_HZ || pclk == hal::soc::chip::UART_BOOT_CLOCK_40M_HZ,
        "unexpected boot UART clock {pclk} Hz",
    );
    let div64 = ((pclk as u64) * 4 / (cfg.baudrate.baud() as u64)) as u32;
    let div = div64 >> 6;
    let exp_div_fra = (div64 & 0x3F) as u16;
    let exp_div_l = (div & 0xFF) as u16;
    let exp_div_h = ((div >> 8) & 0xFF) as u16;

    // SAFETY: read-only MMIO loads of the UART0 divider registers.
    let r = unsafe { &*pac::Uart0::PTR };
    assert_eq!(r.div_l().read().bits(), exp_div_l, "UART0 boot div_l mismatch");
    assert_eq!(r.div_h().read().bits(), exp_div_h, "UART0 boot div_h mismatch");
    assert_eq!(r.div_fra().read().bits(), exp_div_fra, "UART0 boot div_fra mismatch");
}

/// UART blocking write + flush liveness (uart.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn uart0_write_and_flush() {
    use hal::uart::{Config, Uart, UartClock};

    let cfg = Config { clock: UartClock::Boot, ..Config::default() };
    // SAFETY: sequential single-hart run; UART0 singleton not otherwise held.
    let uart = Uart::new_uart0(unsafe { hal::peripherals::Uart0::steal() }, cfg);

    uart.write(b"[hil] uart0_write_and_flush\r\n");
    uart.flush_tx();
    assert!(uart.tx_flushed(), "UART0 TX FIFO did not drain after flush_tx()");
}

/// UART1 TX→RX loopback (uart.rs + io_config.rs). Requires GPIO15 → GPIO16 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback"))]
pub(crate) fn uart1_loopback_tx_to_rx() {
    use hal::io_config::{DriveStrength, IoConfigDriver, MuxFunction, PullResistor, UartPad};
    use hal::uart::{Config, Uart, UartClock};

    let crg = unsafe { &*pac::CldoCrg::PTR };
    crg.cken_ctl1().modify(|r, w| unsafe { w.bits(r.bits() | (1 << 19)) });

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    io.set_uart_mux(UartPad::Uart1Txd, MuxFunction::F1);
    io.set_uart_mux(UartPad::Uart1Rxd, MuxFunction::F1);
    io.configure_uart_pad(UartPad::Uart1Txd, DriveStrength::Strong, PullResistor::None, false, false);
    io.configure_uart_pad(UartPad::Uart1Rxd, DriveStrength::Strong, PullResistor::Up, true, true);

    let cfg = Config { clock: UartClock::Boot, ..Config::default() };
    let uart = Uart::new_uart1(unsafe { hal::peripherals::Uart1::steal() }, cfg);
    while uart.read_byte().is_some() {}
    let sent = 0x5Au8;
    uart.write_byte(sent);
    let mut got = None;
    for _ in 0..2_000_000u32 {
        if let Some(b) = uart.read_byte() {
            got = Some(b);
            break;
        }
    }
    let got = got.expect("UART1 RX got nothing — verify the jumper is on the real UART1 TXD/RXD pads");
    assert_eq!(got, sent, "UART1 loopback mismatch: sent 0x{sent:02x} got 0x{got:02x}");
}
