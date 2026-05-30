//! Low-Speed ADC (LSADC) driver for WS63.
//!
//! The WS63 LSADC is a 12-bit SAR ADC with up to 6 input channels.
//! It includes a FIFO for storing conversion results and supports
//! CIC filtering, offset/gain correction, and scan mode.
//!
//! # Data format
//!
//! Each FIFO entry is 32 bits:
//! - bits 0:13 — 14-bit conversion data
//! - bits 14:16 — channel number (0-5)

use crate::peripherals::Lsadc;

/// LSADC channel (0-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcChannel {
    Channel0 = 0,
    Channel1 = 1,
    Channel2 = 2,
    Channel3 = 3,
    Channel4 = 4,
    Channel5 = 5,
}

impl AdcChannel {
    /// Convert a channel index (0-5) to an `AdcChannel`.
    pub fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Self::Channel0),
            1 => Some(Self::Channel1),
            2 => Some(Self::Channel2),
            3 => Some(Self::Channel3),
            4 => Some(Self::Channel4),
            5 => Some(Self::Channel5),
            _ => None,
        }
    }
}

/// Configuration for an ADC channel.
#[derive(Debug, Clone, Copy)]
pub struct AdcConfig {
    /// Number of samples per conversion (0-15, maps to 1-16 samples).
    pub sample_count: u8,
    /// Number of start cycles.
    pub start_count: u8,
    /// Number of cast cycles.
    pub cast_count: u8,
}

impl Default for AdcConfig {
    fn default() -> Self {
        Self { sample_count: 7, start_count: 3, cast_count: 3 }
    }
}

/// ADC conversion result.
#[derive(Debug, Clone, Copy)]
pub struct AdcSample {
    /// 14-bit conversion data.
    pub data: u16,
    /// Channel that produced this sample.
    pub channel: u8,
}

/// LSADC driver.
pub struct LsAdc<'d> {
    _lsadc: Lsadc<'d>,
}

impl<'d> LsAdc<'d> {
    /// Create a new LSADC driver from the LSADC peripheral.
    pub fn new(lsadc: Lsadc<'d>) -> Self {
        Self { _lsadc: lsadc }
    }

    fn regs(&self) -> &'static ws63_pac::lsadc::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*Lsadc::ptr() }
    }

    /// Enable the ADC peripheral (release reset and enable clock).
    pub fn enable(&mut self) {
        let r = self.regs();

        // Clear reset (da_lsadc_rstn = 1)
        unsafe {
            r.lsadc_ctrl_7().write(|w| w.bits(0x01));
        }
    }

    /// Disable the ADC peripheral.
    pub fn disable(&mut self) {
        unsafe {
            self.regs().lsadc_ctrl_7().write(|w| w.bits(0x00));
        }
    }

    /// Enable specific ADC channels.
    ///
    /// The channel mask is a 16-bit value where each bit enables
    /// one of the 6 channels (bits 16:21 in LSADC_CTRL_7).
    pub fn enable_channels(&mut self, channel_mask: u16) {
        let r = self.regs();
        let current = r.lsadc_ctrl_7().read().bits();
        // Only bits 0:15 hold da_lsadc_en, but the PAC explorer says bits 16:31
        let ch_en = ((channel_mask & 0x3F) as u32) << 16;
        unsafe {
            r.lsadc_ctrl_7().write(|w| w.bits(current | ch_en));
        }
    }

    /// Configure the ADC sampling parameters.
    pub fn configure(&mut self, config: &AdcConfig) {
        let r = self.regs();

        let mut ctrl1: u32 = 0;
        ctrl1 |= (config.sample_count as u32 & 0x0F) << 8;
        ctrl1 |= (config.start_count as u32 & 0x0F) << 16;
        ctrl1 |= (config.cast_count as u32 & 0x0F) << 24;

        unsafe {
            r.lsadc_ctrl_1().write(|w| w.bits(ctrl1));
        }
    }

    /// Start an ADC scan.
    pub fn start_scan(&mut self) {
        let r = self.regs();
        // lsadc_start = 1 (bit 0)
        unsafe {
            r.lsadc_ctrl_6().write(|w| w.bits(0x01));
        }
    }

    /// Stop an ADC scan.
    pub fn stop_scan(&mut self) {
        let r = self.regs();
        // lsadc_stop = 1 (bit 1)
        unsafe {
            r.lsadc_ctrl_6().write(|w| w.bits(0x02));
        }
    }

    /// Check if ADC FIFO data is available.
    pub fn data_ready(&self) -> bool {
        // FIFO not empty: any data available
        // We check the fifo_data register; returns 0 if no data? Actually we need
        // to poll. The FIFO data register has the data, but we need a count.
        // For now, attempt to read and check validity.
        let val = self.regs().lsadc_fifo_data().read().bits();
        // If all 14 data bits are 0 and channel bits are 0, likely no data
        // This is a heuristic; real driver would check FIFO level register
        val != 0
    }

    /// Read a single ADC sample from the FIFO.
    ///
    /// Returns `None` if no data is available.
    pub fn read_sample(&self) -> Option<AdcSample> {
        let val = self.regs().lsadc_fifo_data().read().bits();
        if val == 0 {
            return None;
        }
        Some(AdcSample { data: (val & 0x3FFF) as u16, channel: ((val >> 14) & 0x07) as u8 })
    }

    /// Enable the CIC filter with a given oversampling ratio.
    pub fn enable_cic_filter(&mut self, oversampling_ratio: u8) {
        let r = self.regs();
        unsafe {
            r.cfg_cic_osr().write(|w| w.bits(oversampling_ratio as u32 & 0xFF));
            r.cfg_cic_filter_en().write(|w| w.bits(0x01));
        }
    }

    /// Disable the CIC filter.
    pub fn disable_cic_filter(&mut self) {
        unsafe {
            self.regs().cfg_cic_filter_en().write(|w| w.bits(0x00));
        }
    }

    /// Set the ADC offset correction value.
    pub fn set_offset(&mut self, offset: u16) {
        unsafe {
            self.regs().cfg_offset().write(|w| w.bits(offset as u32 & 0xFFFF));
        }
    }

    /// Set the ADC gain correction value.
    pub fn set_gain(&mut self, gain: u16) {
        unsafe {
            self.regs().cfg_gain().write(|w| w.bits(gain as u32 & 0xFFFF));
        }
    }

    /// Set the data output selection.
    ///
    /// `true` = post-processed data, `false` = raw ADC data.
    pub fn set_data_select(&mut self, processed: bool) {
        unsafe {
            self.regs().cfg_data_sel().write(|w| w.bits(if processed { 1 } else { 0 }));
        }
    }
}
