//! UART driver for WS63 (UART0/1/2, 16C550-compatible with FIFO).
//!
//! Baud rate: div = (div_h << 8 | div_l) + div_fra / 64.
//! Clock source: the 160 MHz PLL-derived UART clock ([`crate::soc::chip::UART_CLOCK_HZ`]),
//! NOT the 240 MHz CPU clock (vendor `clock_init` sets the baud base to 160 MHz).

use crate::peripherals::{Uart0, Uart1, Uart2};
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub baudrate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
}

impl Default for Config {
    fn default() -> Self {
        Self { baudrate: 115200, data_bits: DataBits::Eight, parity: Parity::None, stop_bits: StopBits::One }
    }
}

pub struct Uart<'d, T> {
    _peripheral: PhantomData<&'d T>,
}

#[allow(dead_code)]
fn regs() -> &'static crate::soc::pac::uart0::RegisterBlock {
    // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
    unsafe { &*Uart0::ptr() }
}

fn uart_ptr(idx: u8) -> *const crate::soc::pac::uart0::RegisterBlock {
    match idx {
        0 => Uart0::ptr(),
        1 => Uart1::ptr(),
        2 => Uart2::ptr(),
        _ => unreachable!(),
    }
}

fn uart_regs(idx: u8) -> &'static crate::soc::pac::uart0::RegisterBlock {
    // SAFETY: uart_ptr(idx) returns valid PAC MMIO addresses (UART0/1/2 at 0x4401_0000/1000/2000)
    unsafe { &*uart_ptr(idx) }
}

impl<'d> Uart<'d, Uart0<'d>> {
    pub fn new_uart0(_uart: Uart0<'d>, config: Config) -> Self {
        configure_uart(0, &config);
        Self { _peripheral: PhantomData }
    }
}

impl<'d> Uart<'d, Uart1<'d>> {
    pub fn new_uart1(_uart: Uart1<'d>, config: Config) -> Self {
        configure_uart(1, &config);
        Self { _peripheral: PhantomData }
    }
}

impl<'d> Uart<'d, Uart2<'d>> {
    pub fn new_uart2(_uart: Uart2<'d>, config: Config) -> Self {
        configure_uart(2, &config);
        Self { _peripheral: PhantomData }
    }
}

fn configure_uart(idx: u8, config: &Config) {
    let r = uart_regs(idx);

    // Enable divider access
    r.uart_ctl().modify(|_, w| unsafe { w.bits(0) });
    r.uart_ctl().write(|w| w.div_en().set_bit());

    // Set baud rate: div = UART_CLK / (16 * baudrate)
    // Valid range: div ∈ [1, 65535] (16-bit divider).
    // At 160MHz UART clock, valid baud ∈ [153, 10_000_000].
    let pclk = crate::soc::chip::UART_CLOCK_HZ;
    let min_baud = (pclk / (16 * 65535)) + 1;
    let baudrate = if config.baudrate < min_baud { min_baud } else { config.baudrate };
    // div = pclk / (16 * baud), as fixed-point with 6 fractional bits (div_fra ∈
    // [0,63] sixty-fourths). Dropping the fraction (old div_fra=0) is fine at high
    // clocks but a significant baud error at the TCXO base — flashboot itself
    // programs a non-zero div_fra. (issue #15)
    let div64 = ((pclk as u64) * 4 / (baudrate as u64)) as u32; // = div * 64
    let div = div64 >> 6;
    let div_fra = (div64 & 0x3F) as u16;
    let div_l = (div & 0xFF) as u16;
    let div_h = ((div >> 8) & 0xFF) as u16;
    r.div_l().write(|w| unsafe { w.bits(div_l) });
    r.div_h().write(|w| unsafe { w.bits(div_h) });
    r.div_fra().write(|w| unsafe { w.bits(div_fra) });

    // Configure data bits, parity, stop bits
    let mut ctl = 0u16;
    ctl |= match config.data_bits {
        DataBits::Five => 0,
        DataBits::Six => 1 << 2,
        DataBits::Seven => 2 << 2,
        DataBits::Eight => 3 << 2,
    };
    match config.parity {
        Parity::Even => {
            ctl |= 1 << 5;
            ctl |= 1 << 4;
        }
        Parity::Odd => {
            ctl |= 1 << 5;
        }
        Parity::None => {}
    }
    if matches!(config.stop_bits, StopBits::Two) {
        ctl |= 1 << 7;
    }
    r.uart_ctl().write(|w| unsafe { w.bits(ctl | (1 << 0)) }); // div_en=1

    // Enable FIFO
    r.fifo_ctl().write(|w| unsafe { w.bits(0x01) });

    // Clear FIFO
    r.fifo_ctl().write(|w| unsafe { w.bits(0x07) });
}

impl<T> Uart<'_, T> {
    pub fn write_byte(&self, idx: u8, byte: u8) {
        let r = uart_regs(idx);
        while r.fifo_status().read().tx_fifo_full().bit_is_set() {}
        r.data().write(|w| unsafe { w.bits(byte as u16) });
    }

    pub fn read_byte(&self, idx: u8) -> Option<u8> {
        let r = uart_regs(idx);
        if r.fifo_status().read().rx_fifo_empty().bit_is_set() { None } else { Some(r.data().read().bits() as u8) }
    }

    pub fn flush_tx(&self, idx: u8) {
        let r = uart_regs(idx);
        while !r.fifo_status().read().tx_fifo_empty().bit_is_set() {}
    }

    /// Non-blocking check: returns true if TX FIFO is fully drained.
    pub fn tx_flushed(&self, idx: u8) -> bool {
        uart_regs(idx).fifo_status().read().tx_fifo_empty().bit_is_set()
    }

    pub fn write(&self, idx: u8, data: &[u8]) {
        for &b in data {
            self.write_byte(idx, b);
        }
    }

    pub fn uart_regs(&self, idx: u8) -> &'static crate::soc::pac::uart0::RegisterBlock {
        uart_regs(idx)
    }
}

impl embedded_io::ErrorType for Uart<'_, Uart0<'_>> {
    type Error = core::convert::Infallible;
}
impl embedded_io::Write for Uart<'_, Uart0<'_>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &b in buf {
            self.write_byte(0, b);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_tx(0);
        Ok(())
    }
}
impl embedded_io::Read for Uart<'_, Uart0<'_>> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut n = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.read_byte(0) {
                *b = byte;
                n += 1;
            } else {
                break;
            }
        }
        Ok(n)
    }
}

// UART1 embedded-io traits
impl embedded_io::ErrorType for Uart<'_, Uart1<'_>> {
    type Error = core::convert::Infallible;
}
impl embedded_io::Write for Uart<'_, Uart1<'_>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &b in buf {
            self.write_byte(1, b);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_tx(1);
        Ok(())
    }
}
impl embedded_io::Read for Uart<'_, Uart1<'_>> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut n = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.read_byte(1) {
                *b = byte;
                n += 1;
            } else {
                break;
            }
        }
        Ok(n)
    }
}

// UART2 embedded-io traits
impl embedded_io::ErrorType for Uart<'_, Uart2<'_>> {
    type Error = core::convert::Infallible;
}
impl embedded_io::Write for Uart<'_, Uart2<'_>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &b in buf {
            self.write_byte(2, b);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flush_tx(2);
        Ok(())
    }
}
impl embedded_io::Read for Uart<'_, Uart2<'_>> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut n = 0;
        for b in buf.iter_mut() {
            if let Some(byte) = self.read_byte(2) {
                *b = byte;
                n += 1;
            } else {
                break;
            }
        }
        Ok(n)
    }
}

// ── embedded-hal-nb serial traits ──────────────────────────────

macro_rules! impl_nb_serial {
    ($uart:ty, $idx:expr) => {
        impl embedded_hal_nb::serial::ErrorType for Uart<'_, $uart> {
            type Error = core::convert::Infallible;
        }
        impl embedded_hal_nb::serial::Read for Uart<'_, $uart> {
            fn read(&mut self) -> nb::Result<u8, Self::Error> {
                match self.read_byte($idx) {
                    Some(b) => Ok(b),
                    None => Err(nb::Error::WouldBlock),
                }
            }
        }
        impl embedded_hal_nb::serial::Write for Uart<'_, $uart> {
            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                self.write_byte($idx, byte);
                Ok(())
            }
            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                if self.tx_flushed($idx) { Ok(()) } else { Err(nb::Error::WouldBlock) }
            }
        }
    };
}

impl_nb_serial!(Uart0<'_>, 0);
impl_nb_serial!(Uart1<'_>, 1);
impl_nb_serial!(Uart2<'_>, 2);

// ── Async UART (embedded-io-async) ──────────────────────────────────────────
#[cfg(feature = "async")]
mod asynch_impl {
    use super::{Uart, uart_regs};
    use crate::asynch::IrqSignal;
    use crate::peripherals::{Uart0, Uart1, Uart2};
    use core::cell::Cell;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use critical_section::Mutex;
    use embedded_io_async::{Read, Write};

    static UART_RX: [IrqSignal; 3] = [IrqSignal::new(), IrqSignal::new(), IrqSignal::new()];
    static UART_BYTE: [Mutex<Cell<u8>>; 3] = [const { Mutex::new(Cell::new(0)) }; 3];

    fn uart_base(idx: u8) -> usize {
        match idx {
            0 => 0x4401_0000,
            1 => 0x4401_1000,
            _ => 0x4401_2000,
        }
    }

    /// UART trap hook: read the received byte (which de-asserts the RX IRQ) and
    /// wake the awaiting reader. Call from the trap when `mcause` is UART0..2_INT
    /// (IRQ 53..55). Reading in the ISR avoids a level-triggered RX storm.
    pub fn on_interrupt(idx: u8) {
        let r = uart_regs(idx);
        if !r.fifo_status().read().rx_fifo_empty().bit_is_set() {
            let b = r.data().read().bits() as u8;
            critical_section::with(|cs| UART_BYTE[idx as usize].borrow(cs).set(b));
            UART_RX[idx as usize].signal();
        }
    }

    struct RxFuture {
        idx: u8,
    }
    impl Future for RxFuture {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if UART_RX[self.idx as usize].take_fired() {
                Poll::Ready(())
            } else {
                UART_RX[self.idx as usize].register(cx.waker());
                Poll::Pending
            }
        }
    }

    async fn read_one(uart: &Uart<'_, impl Sized>, idx: u8) -> u8 {
        let r = uart_regs(idx);
        if !r.fifo_status().read().rx_fifo_empty().bit_is_set() {
            return r.data().read().bits() as u8; // byte already waiting
        }
        UART_RX[idx as usize].reset();
        // Enable the UART RX interrupt (INTR_EN @ +0x18) so a byte raises the IRQ.
        unsafe { core::ptr::write_volatile((uart_base(idx) + 0x18) as *mut u32, 1) };
        let _ = uart; // keep the &Uart borrow for the await's lifetime
        RxFuture { idx }.await;
        critical_section::with(|cs| UART_BYTE[idx as usize].borrow(cs).get())
    }

    macro_rules! async_uart {
        ($uart:ty, $idx:expr) => {
            // embedded_io::ErrorType is already implemented for the blocking impls;
            // the async Read/Write traits reuse it.
            impl Write for Uart<'_, $uart> {
                async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
                    // WS63 UART TX drains immediately on this model; write_byte polls
                    // tx_fifo_full, so this completes without parking.
                    for &b in buf {
                        self.write_byte($idx, b);
                    }
                    Ok(buf.len())
                }
                async fn flush(&mut self) -> Result<(), Self::Error> {
                    self.flush_tx($idx);
                    Ok(())
                }
            }
            impl Read for Uart<'_, $uart> {
                async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
                    if buf.is_empty() {
                        return Ok(0);
                    }
                    buf[0] = read_one(self, $idx).await;
                    Ok(1)
                }
            }
        };
    }
    async_uart!(Uart0<'_>, 0);
    async_uart!(Uart1<'_>, 1);
    async_uart!(Uart2<'_>, 2);
}

#[cfg(feature = "async")]
pub use asynch_impl::on_interrupt;
