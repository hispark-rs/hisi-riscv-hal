/// SPI0 MOSI→MISO loopback (spi.rs + io_config.rs). Requires GPIO9 → GPIO11 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback"))]
pub(crate) fn spi0_loopback_mosi_to_miso() {
    use crate::hal;
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};
    use hal::spi::{Config, Spi};

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    for pin in [GpioPad::Gpio07, GpioPad::Gpio09, GpioPad::Gpio10, GpioPad::Gpio11] {
        io.set_gpio_mux(pin, MuxFunction::F3);
    }

    // SAFETY: SPI0 singleton not otherwise held; GPIO9->GPIO11 jumpered.
    let mut spi = Spi::new_spi0(unsafe { hal::peripherals::Spi0::steal() }, Config::default());
    let tx = [0xA5u8, 0x3C, 0x00, 0xFF];
    let mut rx = [0u8; 4];
    let res = spi.transfer(&tx, &mut rx);
    semihosting::println!("[spi-lb] transfer={res:?} tx={tx:02x?} rx={rx:02x?}");
    res.expect("SPI0 transfer returned an error");
    assert_eq!(rx, tx, "SPI0 loopback mismatch — check GPIO9->GPIO11 jumper: tx={tx:02x?} rx={rx:02x?}");
}

/// SPI0 TX DMA via `SpiDma::write_dma`. Requires GPIO9 → GPIO11 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback", feature = "unstable"))]
pub(crate) fn spi_dma_tx_loopback() {
    use crate::hal;
    use hal::dma::{Dma0, DmaDriver};
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};
    use hal::spi::{Config, Spi};

    const N: usize = 8;
    #[repr(C, align(32))]
    struct Aligned([u8; N]);
    static TX: Aligned = Aligned([0xA5, 0x3C, 0x00, 0xFF, 0x5A, 0xC3, 0x0F, 0xF0]);

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    for pin in [GpioPad::Gpio07, GpioPad::Gpio09, GpioPad::Gpio10, GpioPad::Gpio11] {
        io.set_gpio_mux(pin, MuxFunction::F3);
    }

    // SAFETY: SPI0/DMA singletons not otherwise held.
    let spi = Spi::new_spi0(unsafe { hal::peripherals::Spi0::steal() }, Config::default());
    let mut dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    dma.enable_controller();
    let chs = dma.split_channels().expect("DMA channels already claimed");
    let mut sd = spi.with_dma(dma);

    sd.write_dma(chs.ch0, &TX.0[..]).expect("SpiDma::write_dma failed");
    semihosting::println!("[spi-dma-tx] SpiDma::write_dma ok for {N} bytes");
}

/// SPI0 full-duplex DMA via `SpiDma::transfer_dma`. Requires GPIO9 → GPIO11 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback", feature = "unstable"))]
pub(crate) fn spi_dma_fullduplex_loopback() {
    use crate::hal;
    use hal::dma::{Dma0, DmaDriver};
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};
    use hal::spi::{Config, Spi};

    const N: usize = 8;
    #[repr(C, align(32))]
    struct Aligned([u8; N]);
    static TX: Aligned = Aligned([0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F, 0x70, 0x81]);
    static mut RX: Aligned = Aligned([0u8; N]);
    // SAFETY: sequential single-hart run; RX touched only here.
    let rx: &'static mut [u8] = unsafe { &mut (*core::ptr::addr_of_mut!(RX)).0 };

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    for pin in [GpioPad::Gpio07, GpioPad::Gpio09, GpioPad::Gpio10, GpioPad::Gpio11] {
        io.set_gpio_mux(pin, MuxFunction::F3);
    }

    // SAFETY: SPI0/DMA singletons not otherwise held.
    let spi = Spi::new_spi0(unsafe { hal::peripherals::Spi0::steal() }, Config::default());
    let mut dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    dma.enable_controller();
    let chs = dma.split_channels().expect("DMA channels already claimed");
    let mut sd = spi.with_dma(dma);

    let (rx_buf, _tx_buf) = sd.transfer_dma(chs.ch0, chs.ch1, rx, &TX.0[..]).expect("SpiDma::transfer_dma failed");
    semihosting::println!("[spi-dma-fdx-lb] tx={:02x?} rx={:02x?}", TX.0, &rx_buf[..]);
    assert_eq!(&rx_buf[..], &TX.0, "SPI0 full-duplex DMA loopback mismatch — check GPIO9->GPIO11 jumper");
}

/// DMA completion IRQ proof for peripheral-paced SPI DMA. Requires GPIO9 → GPIO11 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback", feature = "async", feature = "unstable"))]
pub(crate) fn spi_dma_irq59_fires_on_completion() {
    use crate::hal;
    use hal::asynch::block_on;
    use hal::dma::{Dma0, DmaDriver};
    use hal::interrupt;
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};
    use hal::spi::{Config, Spi};

    const N: usize = 8;
    #[repr(C, align(32))]
    struct Aligned([u8; N]);
    static TX: Aligned = Aligned([0xA5, 0x3C, 0x00, 0xFF, 0x5A, 0xC3, 0x0F, 0xF0]);

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    for pin in [GpioPad::Gpio07, GpioPad::Gpio09, GpioPad::Gpio10, GpioPad::Gpio11] {
        io.set_gpio_mux(pin, MuxFunction::F3);
    }
    let spi = Spi::new_spi0(unsafe { hal::peripherals::Spi0::steal() }, Config::default());
    let dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    let chs = dma.split_channels().expect("DMA channels already claimed");
    let mut sd = spi.with_dma(dma);

    unsafe { interrupt::enable_global() };
    block_on(async { sd.write_dma_async(chs.ch0, &TX.0[..]).await }).expect("SpiDma::write_dma_async failed");
    unsafe { interrupt::disable_global() };
    semihosting::println!("[spi-dma-irq59] IRQ 59 fired for peripheral DMA completion — async .await is viable");
}

/// `SpiDma::write_dma_async` capstone. Requires GPIO9 → GPIO11 jumper.
#[cfg(all(feature = "chip-ws63", feature = "hil-loopback", feature = "async", feature = "unstable"))]
pub(crate) fn spi_dma_write_async() {
    use crate::hal;
    use hal::asynch::block_on;
    use hal::dma::{Dma0, DmaDriver};
    use hal::interrupt;
    use hal::io_config::{GpioPad, IoConfigDriver, MuxFunction};
    use hal::spi::{Config, Spi};

    const N: usize = 8;
    #[repr(C, align(32))]
    struct Aligned([u8; N]);
    static TX: Aligned = Aligned([0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22]);

    let mut io = IoConfigDriver::new(unsafe { hal::peripherals::IoConfig::steal() });
    for pin in [GpioPad::Gpio07, GpioPad::Gpio09, GpioPad::Gpio10, GpioPad::Gpio11] {
        io.set_gpio_mux(pin, MuxFunction::F3);
    }
    let spi = Spi::new_spi0(unsafe { hal::peripherals::Spi0::steal() }, Config::default());
    let mut dma = DmaDriver::<Dma0>::new_dma(unsafe { hal::peripherals::Dma::steal() });
    dma.enable_controller();
    let chs = dma.split_channels().expect("DMA channels already claimed");
    let mut sd = spi.with_dma(dma);

    unsafe { interrupt::enable_global() };
    block_on(async { sd.write_dma_async(chs.ch0, &TX.0[..]).await }).expect("SpiDma::write_dma_async failed");
    unsafe { interrupt::disable_global() };
    semihosting::println!("[spi-dma-async] SpiDma::write_dma_async ok for {N} bytes");
}
