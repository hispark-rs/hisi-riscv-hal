//! True Random Number Generator (TRNG) driver for WS63.
//!
//! The WS63 TRNG generates true random numbers using physical entropy
//! sources (FRO — Free-Running Oscillator). Random data is read from
//! a FIFO register.
//!
//! # Usage
//!
//! ```ignore
//! let trng = Trng::new(peripherals.TRNG);
//! let random_word: u32 = trng.read().unwrap();
//! ```

use crate::peripherals::Trng;

/// True Random Number Generator driver.
pub struct TrngDriver<'d> {
    _trng: Trng<'d>,
}

impl<'d> TrngDriver<'d> {
    /// Create a new TRNG driver.
    pub fn new(trng: Trng<'d>) -> Self {
        Self { _trng: trng }
    }

    fn regs(&self) -> &'static ws63_pac::trng::RegisterBlock {
        unsafe { &*Trng::ptr() }
    }

    /// Check if random data is ready in the FIFO.
    pub fn data_ready(&self) -> bool {
        self.regs().trng_fifo_ready().read().bits() & 0x01 != 0
    }

    /// Check if the TRNG generation is complete.
    pub fn done(&self) -> bool {
        self.regs().trng_fifo_ready().read().bits() & 0x02 != 0
    }

    /// Read a 32-bit random word from the TRNG FIFO.
    ///
    /// Returns `None` if no data is available.
    pub fn read(&self) -> Option<u32> {
        if !self.data_ready() {
            return None;
        }
        Some(self.regs().trng_fifo_data().read().bits())
    }

    /// Read a random word, blocking until data is available.
    pub fn read_blocking(&self) -> u32 {
        while !self.data_ready() {}
        self.regs().trng_fifo_data().read().bits()
    }

    /// Fill a buffer with random bytes.
    ///
    /// Each 32-bit word produces 4 bytes.
    pub fn fill_bytes(&self, buf: &mut [u8]) {
        let mut i = 0;
        while i < buf.len() {
            let word = self.read_blocking();
            let bytes = word.to_le_bytes();
            for &b in &bytes {
                if i < buf.len() {
                    buf[i] = b;
                    i += 1;
                }
            }
        }
    }

    /// Fill a buffer with random 32-bit words.
    pub fn fill_words(&self, buf: &mut [u32]) {
        for word in buf.iter_mut() {
            *word = self.read_blocking();
        }
    }

    /// Select the FRO sample clock source.
    ///
    /// * `external` — `true` for external clock, `false` for internal clock.
    pub fn set_sample_clock(&mut self, external: bool) {
        unsafe {
            self.regs().trng_fro_sample_clk_sel().write(|w| w.bits(if external { 1 } else { 0 }));
        }
    }

    /// Set the FRO divider count.
    ///
    /// Controls the sampling rate of the FRO entropy source.
    /// Default is 0x1b (27).
    pub fn set_divider(&mut self, div: u8) {
        unsafe {
            self.regs().trng_fro_div_cnt().write(|w| w.bits(div as u32));
        }
    }

    /// Get the data status register value (for debugging).
    pub fn data_status(&self) -> u32 {
        self.regs().trng_data_st().read().bits()
    }
}
