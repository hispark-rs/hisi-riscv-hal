//! I2S (Inter-IC Sound) / PCM audio interface driver for WS63.
//!
//! The WS63 I2S peripheral supports both I2S and PCM modes, master and slave
//! operation, 2-8 channels, configurable data widths, and separate TX/RX
//! FIFOs with threshold interrupts.
//!
//! # Clock configuration
//!
//! In master mode, BCLK and FS (frame sync) are generated from the system
//! clock via programmable dividers.
//!
//! * BCLK = I2S_CLK / (BCLK_DIV_NUM + 1)
//! * FS = BCLK / (FS_DIV_NUM + 1)

use crate::peripherals::I2s;

/// I2S operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2sMode {
    /// I2S (Philips) mode.
    I2s,
    /// PCM mode.
    Pcm,
}

/// Master/slave role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2sRole {
    /// Slave (receives BCLK and FS externally).
    Slave,
    /// Master (generates BCLK and FS).
    Master,
}

/// Clock edge selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEdge {
    /// Rising edge.
    Rising,
    /// Falling edge.
    Falling,
}

/// Number of audio channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelCount {
    /// 2 channels (stereo).
    Two = 0,
    /// 4 channels.
    Four = 1,
    /// 6 channels.
    Six = 2,
    /// 8 channels.
    Eight = 3,
}

/// TX/RX data width mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataWidth {
    /// 8-bit data.
    Bits8 = 0,
    /// 10-bit data.
    Bits10 = 1,
    /// 12-bit data.
    Bits12 = 2,
    /// 14-bit data.
    Bits14 = 3,
    /// 16-bit data.
    Bits16 = 4,
    /// 18-bit data.
    Bits18 = 5,
    /// 20-bit data.
    Bits20 = 6,
    /// 24-bit data.
    Bits24 = 7,
}

/// I2S configuration.
#[derive(Debug, Clone, Copy)]
pub struct I2sConfig {
    /// I2S or PCM mode.
    pub mode: I2sMode,
    /// Master or slave.
    pub role: I2sRole,
    /// Clock edge (only used in slave mode).
    pub clock_edge: ClockEdge,
    /// Number of channels.
    pub channels: ChannelCount,
    /// TX data width.
    pub tx_width: DataWidth,
    /// RX data width.
    pub rx_width: DataWidth,
    /// BCLK divider in master mode (0-127).
    pub bclk_div: u8,
    /// FS divider numerator in master mode (0-1023).
    pub fs_div_num: u16,
    /// FS divider ratio in master mode (0-2047).
    pub fs_div_ratio: u16,
    /// TX FIFO threshold.
    pub tx_fifo_threshold: u8,
    /// RX FIFO threshold.
    pub rx_fifo_threshold: u8,
    /// Enable loopback mode (TX → RX internally).
    pub loopback: bool,
}

impl Default for I2sConfig {
    fn default() -> Self {
        Self {
            mode: I2sMode::I2s,
            role: I2sRole::Master,
            clock_edge: ClockEdge::Rising,
            channels: ChannelCount::Two,
            tx_width: DataWidth::Bits16,
            rx_width: DataWidth::Bits16,
            bclk_div: 0,
            fs_div_num: 0,
            fs_div_ratio: 0,
            tx_fifo_threshold: 8,
            rx_fifo_threshold: 8,
            loopback: false,
        }
    }
}

/// I2S audio interface driver.
pub struct I2sDriver<'d> {
    _i2s: I2s<'d>,
}

impl<'d> I2sDriver<'d> {
    /// Create a new I2S driver.
    pub fn new(i2s: I2s<'d>) -> Self {
        Self { _i2s: i2s }
    }

    fn regs(&self) -> &'static crate::soc::pac::i2s::RegisterBlock {
        // SAFETY: PAC peripheral pointer is a static physical MMIO address, always valid
        unsafe { &*I2s::ptr() }
    }

    /// Configure the I2S interface.
    pub fn configure(&mut self, config: &I2sConfig) {
        let r = self.regs();

        // Set mode register
        let mut mode: u32 = 0;
        mode |= (config.channels as u32) & 0x03; // channels [0:1]
        if matches!(config.clock_edge, ClockEdge::Falling) {
            mode |= 1 << 2; // clk_edge
        }
        if matches!(config.role, I2sRole::Master) {
            mode |= 1 << 3; // master_slave
        }
        if matches!(config.mode, I2sMode::Pcm) {
            mode |= 1 << 4; // pcm_mode
        }
        unsafe {
            r.mode().write(|w| w.bits(mode));
        }

        // Set loopback mode
        if config.loopback {
            unsafe {
                r.version().write(|w| w.bits(1 << 8));
            }
        }

        // Set data width modes
        let dw = ((config.tx_width as u32) & 0x07) | (((config.rx_width as u32) & 0x07) << 8);
        unsafe {
            r.data_width_set().write(|w| w.bits(dw));
        }

        // Set FIFO thresholds
        let thresh = (config.tx_fifo_threshold as u32 & 0xFF) | ((config.rx_fifo_threshold as u32 & 0xFF) << 8);
        unsafe {
            r.fifo_threshold().write(|w| w.bits(thresh));
        }

        // Set clock dividers (for master mode)
        unsafe {
            r.i2s_bclk_div_num().write(|w| w.bits(config.bclk_div as u32 & 0x7F));
            r.i2s_fs_div_num().write(|w| w.bits(config.fs_div_num as u32 & 0x3FF));
            r.i2s_fs_div_ratio_num().write(|w| w.bits(config.fs_div_ratio as u32 & 0x7FF));
        }

        // Enable clock in master mode
        let crg_val = if matches!(config.role, I2sRole::Master) {
            0x100 // clk_en = 1
        } else {
            0
        };
        unsafe {
            r.i2s_crg().write(|w| w.bits(crg_val));
        }

        // Enable signed extension (for correct audio sample handling)
        unsafe {
            r.signed_ext().write(|w| w.bits(0x01));
        }
    }

    /// Enable TX.
    pub fn enable_tx(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x01)); // tx_en
        }
    }

    /// Enable RX.
    pub fn enable_rx(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x02)); // rx_en
        }
    }

    /// Disable TX.
    pub fn disable_tx(&mut self) {
        unsafe {
            self.regs().ct_clr().write(|w| w.bits(0x01));
        }
    }

    /// Disable RX.
    pub fn disable_rx(&mut self) {
        unsafe {
            self.regs().ct_clr().write(|w| w.bits(0x02));
        }
    }

    /// Reset TX FIFO.
    pub fn reset_tx(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x04));
        }
    }

    /// Reset RX FIFO.
    pub fn reset_rx(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x08));
        }
    }

    /// Enable TX interrupt.
    pub fn enable_tx_interrupt(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x10));
        }
    }

    /// Enable RX interrupt.
    pub fn enable_rx_interrupt(&mut self) {
        unsafe {
            self.regs().ct_set().write(|w| w.bits(0x20));
        }
    }

    /// Write a sample to the left TX channel.
    pub fn write_left(&mut self, data: u32) {
        unsafe {
            self.regs().left_tx().write(|w| w.bits(data));
        }
    }

    /// Write a sample to the right TX channel.
    pub fn write_right(&mut self, data: u32) {
        unsafe {
            self.regs().right_tx().write(|w| w.bits(data));
        }
    }

    /// Read a sample from the left RX channel.
    pub fn read_left(&self) -> u32 {
        self.regs().left_rx().read().bits()
    }

    /// Read a sample from the right RX channel.
    pub fn read_right(&self) -> u32 {
        self.regs().right_rx().read().bits()
    }

    /// Get TX FIFO depth (left channel).
    pub fn tx_fifo_left_depth(&self) -> u8 {
        (self.regs().tx_sta().read().bits() & 0xFF) as u8
    }

    /// Get TX FIFO depth (right channel).
    pub fn tx_fifo_right_depth(&self) -> u8 {
        ((self.regs().tx_sta().read().bits() >> 8) & 0xFF) as u8
    }

    /// Get RX FIFO depth (left channel).
    pub fn rx_fifo_left_depth(&self) -> u8 {
        (self.regs().rx_sta().read().bits() & 0xFF) as u8
    }

    /// Get RX FIFO depth (right channel).
    pub fn rx_fifo_right_depth(&self) -> u8 {
        ((self.regs().rx_sta().read().bits() >> 8) & 0xFF) as u8
    }

    /// Check interrupt status.
    ///
    /// Returns `(rx_int, tx_int, rx_overflow, tx_underflow)`.
    pub fn interrupt_status(&self) -> (bool, bool, bool, bool) {
        let sts = self.regs().intstatus().read().bits();
        ((sts & 0x01) != 0, (sts & 0x02) != 0, (sts & 0x04) != 0, (sts & 0x08) != 0)
    }

    /// Clear all interrupts.
    pub fn clear_interrupts(&mut self) {
        unsafe {
            self.regs().intclr().write(|w| w.bits(0x0F));
        }
    }

    /// Set the interrupt mask.
    ///
    /// * `rx_int_mask` — Mask RX interrupt
    /// * `tx_int_mask` — Mask TX interrupt
    /// * `rx_overflow_mask` — Mask RX overflow interrupt
    /// * `tx_underflow_mask` — Mask TX underflow interrupt
    pub fn set_interrupt_mask(
        &mut self,
        rx_int_mask: bool,
        tx_int_mask: bool,
        rx_overflow_mask: bool,
        tx_underflow_mask: bool,
    ) {
        let mut val: u32 = 0;
        if rx_int_mask {
            val |= 0x01;
        }
        if tx_int_mask {
            val |= 0x02;
        }
        if rx_overflow_mask {
            val |= 0x04;
        }
        if tx_underflow_mask {
            val |= 0x08;
        }
        unsafe {
            self.regs().intmask().write(|w| w.bits(val));
        }
    }

    /// Get the I2S IP version.
    pub fn version(&self) -> u8 {
        (self.regs().version().read().bits() & 0xFF) as u8
    }
}
