//! SPI master driver for WS63 (SPI0/1, SSI v151).
//! DesignWare SSI: SCK = SSI_CLK / SCKDV, default SSI_CLK = 240MHz (SCKDV is even, >= 2).

use crate::peripherals::{Spi0, Spi1};
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub frequency: u32,
    pub mode: SpiMode,
    pub data_bits: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self { frequency: 1_000_000, mode: SpiMode::Mode0, data_bits: 8 }
    }
}

pub struct Spi<'d, T> {
    idx: u8,
    _peripheral: PhantomData<&'d T>,
}

fn spi_regs(idx: u8) -> &'static ws63_pac::spi0::RegisterBlock {
    unsafe {
        match idx {
            0 => &*Spi0::ptr(),
            1 => &*Spi1::ptr(),
            _ => unreachable!(),
        }
    }
}

/// SCKDV clock divider for the DesignWare SSI: `SCK = SSI_CLK / SCKDV`.
///
/// Matches the WS63 C SDK (`hal_spi_v151.c`: `clk_div = bus_clk / freq`,
/// clamped to `SPI_MINUMUM_CLK_DIV` = 2). SCKDV bit 0 is read-only 0, so the
/// value must be even. There is NO `/2` and NO `-1` (an earlier version of this
/// driver had both, producing ~2x the requested SCK).
fn sckdv(pclk: u32, freq: u32) -> u32 {
    let freq = if freq == 0 { 1 } else { freq };
    let div = (pclk / freq).clamp(2, 0xFFFF);
    div & !1 // SCKDV LSB is read-only 0 (must be even)
}

/// Bounded busy-wait. Returns [`SpiError::Timeout`] instead of hanging the CPU
/// forever if a status bit never asserts (no slave, stuck CS, wrong mode) — the
/// C SDK guards every equivalent wait with `hal_spi_check_timeout_by_count`.
const SPI_WAIT_LOOPS: u32 = 1_000_000;

#[inline]
fn wait_until(mut ready: impl FnMut() -> bool) -> Result<(), SpiError> {
    let mut n = SPI_WAIT_LOOPS;
    while !ready() {
        n -= 1;
        if n == 0 {
            return Err(SpiError::Timeout);
        }
    }
    Ok(())
}

impl<'d> Spi<'d, Spi0<'d>> {
    pub fn new_spi0(_spi: Spi0<'d>, config: Config) -> Self {
        configure_spi(0, &config);
        Self { idx: 0, _peripheral: PhantomData }
    }
}
impl<'d> Spi<'d, Spi1<'d>> {
    pub fn new_spi1(_spi: Spi1<'d>, config: Config) -> Self {
        configure_spi(1, &config);
        Self { idx: 1, _peripheral: PhantomData }
    }
}

fn configure_spi(idx: u8, config: &Config) {
    let r = spi_regs(idx);
    r.spi_er().write(|w| unsafe { w.bits(0) });
    let pclk = crate::soc::ws63::SYSTEM_CLOCK_HZ;
    r.spi_brs().write(|w| unsafe { w.bits(sckdv(pclk, config.frequency)) });

    let mut ctra = 0u32;
    match config.mode {
        SpiMode::Mode0 => {}
        SpiMode::Mode1 => ctra |= 1 << 3,
        SpiMode::Mode2 => ctra |= 1 << 4,
        SpiMode::Mode3 => ctra |= (1 << 3) | (1 << 4),
    }
    ctra |= ((config.data_bits.saturating_sub(1)) as u32) << 13;
    // CTRA.trsm (bits 18:19): 0b00 = transmit-and-receive (full duplex).
    // (0b11 is EEPROM-read, NOT TX+RX — leaving trsm = 0 is correct.)
    r.spi_ctra().write(|w| unsafe { w.bits(ctra) });
    r.spi_slenr().write(|w| unsafe { w.bits(0x1) });
    r.spi_er().write(|w| unsafe { w.bits(0x1) });
}

impl<T> Spi<'_, T> {
    pub fn transfer(&mut self, write: &[u8], read: &mut [u8]) -> Result<(), SpiError> {
        let r = spi_regs(self.idx);
        let len = write.len().max(read.len());
        for i in 0..len {
            let tx = if i < write.len() { write[i] as u32 } else { 0 };
            wait_until(|| r.spi_wsr().read().txfnf().bit_is_set())?;
            unsafe { r.spi_dr().write(|w| w.bits(tx)) };
            wait_until(|| r.spi_wsr().read().rxfne().bit_is_set())?;
            let rx = r.spi_dr().read().bits();
            if i < read.len() {
                read[i] = rx as u8;
            }
        }
        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), SpiError> {
        let r = spi_regs(self.idx);
        for &byte in data {
            wait_until(|| r.spi_wsr().read().txfnf().bit_is_set())?;
            unsafe { r.spi_dr().write(|w| w.bits(byte as u32)) };
        }
        Ok(())
    }

    pub fn register_block(&self) -> &'static ws63_pac::spi0::RegisterBlock {
        spi_regs(self.idx)
    }

    /// Wait for all pending TX data to be transmitted and the SPI bus to become idle.
    pub fn wait_idle(&self) -> Result<(), SpiError> {
        let r = spi_regs(self.idx);
        // Wait for TX FIFO to drain, then for the bus to leave the busy state.
        wait_until(|| r.spi_wsr().read().txfe().bit_is_set())?;
        wait_until(|| !r.spi_wsr().read().busy().bit_is_set())?;
        Ok(())
    }
}

fn transfer_in_place_on(idx: u8, buf: &mut [u8]) -> Result<(), SpiError> {
    let r = spi_regs(idx);
    for byte in buf.iter_mut() {
        let tx = *byte as u32;
        wait_until(|| r.spi_wsr().read().txfnf().bit_is_set())?;
        unsafe { r.spi_dr().write(|w| w.bits(tx)) };
        wait_until(|| r.spi_wsr().read().rxfne().bit_is_set())?;
        *byte = r.spi_dr().read().bits() as u8;
    }
    Ok(())
}

#[derive(Debug)]
pub enum SpiError {
    Overflow,
    Timeout,
}

impl embedded_hal::spi::Error for SpiError {
    fn kind(&self) -> embedded_hal::spi::ErrorKind {
        embedded_hal::spi::ErrorKind::Other
    }
}
impl embedded_hal::spi::ErrorType for Spi<'_, Spi0<'_>> {
    type Error = SpiError;
}
impl embedded_hal::spi::SpiBus for Spi<'_, Spi0<'_>> {
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.transfer(write, read)
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(&[], buf)
    }
    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        Spi::write(self, buf)
    }
    fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        transfer_in_place_on(self.idx, buf)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.wait_idle()
    }
}

// SPI1 also implements the same traits
impl embedded_hal::spi::ErrorType for Spi<'_, Spi1<'_>> {
    type Error = SpiError;
}
impl embedded_hal::spi::SpiBus for Spi<'_, Spi1<'_>> {
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.transfer(write, read)
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.transfer(&[], buf)
    }
    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        Spi::write(self, buf)
    }
    fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        transfer_in_place_on(self.idx, buf)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.wait_idle()
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::sckdv;
    use crate::soc::ws63::SYSTEM_CLOCK_HZ;

    #[test]
    fn test_sckdv_basic() {
        // 240 MHz / 1 MHz = 240 (SDK writes the divisor directly, no /2, no -1).
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, 1_000_000), 240);
    }

    #[test]
    fn test_sckdv_is_even_and_min_two() {
        // SCKDV bit0 is read-only 0 → result always even.
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, 1_000_000) & 1, 0);
        // freq >= pclk clamps to the minimum divisor of 2.
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, SYSTEM_CLOCK_HZ), 2);
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, u32::MAX), 2);
    }

    #[test]
    fn test_sckdv_zero_freq_guard() {
        // freq == 0 is treated as 1 Hz → very large divisor, clamped to even max.
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, 0), 0xFFFE);
    }

    #[test]
    fn test_sckdv_clamps_at_max() {
        assert_eq!(sckdv(SYSTEM_CLOCK_HZ, 1000), 0xFFFE);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(test)]
mod proptests {
    use super::sckdv;
    use proptest::prelude::*;

    proptest! {
        /// Fuzz: sckdv never panics and is always a valid even divisor in [2, 0xFFFE].
        #[test]
        fn sckdv_in_valid_range(freq in any::<u32>()) {
            let d = sckdv(crate::soc::ws63::SYSTEM_CLOCK_HZ, freq);
            prop_assert!((2..=0xFFFE).contains(&d), "divisor {} out of range for freq={}", d, freq);
            prop_assert_eq!(d & 1, 0, "divisor {} not even for freq={}", d, freq);
        }

        /// Fuzz: higher frequency → lower-or-equal divisor (monotonic non-increasing).
        #[test]
        fn sckdv_monotonic(freq1 in 1u32.., freq2 in 1u32..) {
            let pclk = crate::soc::ws63::SYSTEM_CLOCK_HZ;
            let d1 = sckdv(pclk, freq1);
            let d2 = sckdv(pclk, freq2);
            if freq1 > freq2 {
                prop_assert!(d1 <= d2, "freq1={}(d={}) freq2={}(d={})", freq1, d1, freq2, d2);
            }
        }
    }
}
