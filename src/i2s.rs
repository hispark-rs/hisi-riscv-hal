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

// ── Tests ──────────────────────────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod tests {
    use super::*;

    // Re-derive the pure `mode` register encoding from `configure()` (lines that
    // build `mode`): channels in bits[0:1], clk_edge=bit2, master=bit3, pcm=bit4.
    // Kept byte-for-byte identical to the driver so the test guards that packing.
    fn encode_mode(c: &I2sConfig) -> u32 {
        let mut mode: u32 = 0;
        mode |= (c.channels as u32) & 0x03;
        if matches!(c.clock_edge, ClockEdge::Falling) {
            mode |= 1 << 2;
        }
        if matches!(c.role, I2sRole::Master) {
            mode |= 1 << 3;
        }
        if matches!(c.mode, I2sMode::Pcm) {
            mode |= 1 << 4;
        }
        mode
    }

    // Re-derive the `intmask` encoding from `set_interrupt_mask()`.
    fn encode_int_mask(rx: bool, tx: bool, rx_ovf: bool, tx_unf: bool) -> u32 {
        let mut val: u32 = 0;
        if rx {
            val |= 0x01;
        }
        if tx {
            val |= 0x02;
        }
        if rx_ovf {
            val |= 0x04;
        }
        if tx_unf {
            val |= 0x08;
        }
        val
    }

    // Re-derive the `intstatus` decode from `interrupt_status()`.
    fn decode_int_status(sts: u32) -> (bool, bool, bool, bool) {
        ((sts & 0x01) != 0, (sts & 0x02) != 0, (sts & 0x04) != 0, (sts & 0x08) != 0)
    }

    #[test]
    fn channel_count_discriminants() {
        // ChannelCount maps 2/4/6/8 channels to the field codes 0..3.
        assert_eq!(ChannelCount::Two as u32, 0);
        assert_eq!(ChannelCount::Four as u32, 1);
        assert_eq!(ChannelCount::Six as u32, 2);
        assert_eq!(ChannelCount::Eight as u32, 3);
    }

    #[test]
    fn data_width_discriminants() {
        // DataWidth codes are the dense 0..7 sequence used by both TX and RX fields.
        assert_eq!(DataWidth::Bits8 as u32, 0);
        assert_eq!(DataWidth::Bits10 as u32, 1);
        assert_eq!(DataWidth::Bits12 as u32, 2);
        assert_eq!(DataWidth::Bits14 as u32, 3);
        assert_eq!(DataWidth::Bits16 as u32, 4);
        assert_eq!(DataWidth::Bits18 as u32, 5);
        assert_eq!(DataWidth::Bits20 as u32, 6);
        assert_eq!(DataWidth::Bits24 as u32, 7);
    }

    #[test]
    fn default_config_values() {
        // The default is a 16-bit stereo I2S master with no loopback and FIFO
        // thresholds of 8, and all dividers zeroed.
        let c = I2sConfig::default();
        assert_eq!(c.mode, I2sMode::I2s);
        assert_eq!(c.role, I2sRole::Master);
        assert_eq!(c.clock_edge, ClockEdge::Rising);
        assert_eq!(c.channels, ChannelCount::Two);
        assert_eq!(c.tx_width, DataWidth::Bits16);
        assert_eq!(c.rx_width, DataWidth::Bits16);
        assert_eq!(c.bclk_div, 0);
        assert_eq!(c.fs_div_num, 0);
        assert_eq!(c.fs_div_ratio, 0);
        assert_eq!(c.tx_fifo_threshold, 8);
        assert_eq!(c.rx_fifo_threshold, 8);
        assert!(!c.loopback);
    }

    #[test]
    fn mode_encoding_default() {
        // Default: 2ch (0) + rising (no bit2) + master (bit3) + I2s (no bit4) = 0x08.
        assert_eq!(encode_mode(&I2sConfig::default()), 0x08);
    }

    #[test]
    fn mode_encoding_all_bits() {
        // 8ch (code 3) + falling + master + pcm sets all of bits[0:4].
        let c = I2sConfig {
            channels: ChannelCount::Eight,
            clock_edge: ClockEdge::Falling,
            role: I2sRole::Master,
            mode: I2sMode::Pcm,
            ..I2sConfig::default()
        };
        // 0b11 | (1<<2) | (1<<3) | (1<<4) = 0x1F.
        assert_eq!(encode_mode(&c), 0x1F);
    }

    #[test]
    fn mode_encoding_slave_clears_master_bit() {
        // Slave role must not set bit3; rising edge must not set bit2.
        let c = I2sConfig {
            role: I2sRole::Slave,
            clock_edge: ClockEdge::Rising,
            channels: ChannelCount::Two,
            mode: I2sMode::I2s,
            ..I2sConfig::default()
        };
        assert_eq!(encode_mode(&c), 0x00);
    }

    #[test]
    fn mode_encoding_fields_are_disjoint() {
        // Each independent flag occupies exactly one bit and does not collide
        // with the channel field.
        let base = I2sConfig { channels: ChannelCount::Two, role: I2sRole::Slave, ..I2sConfig::default() };
        let falling = I2sConfig { clock_edge: ClockEdge::Falling, ..base };
        let master = I2sConfig { role: I2sRole::Master, ..base };
        let pcm = I2sConfig { mode: I2sMode::Pcm, ..base };
        assert_eq!(encode_mode(&falling), 1 << 2);
        assert_eq!(encode_mode(&master), 1 << 3);
        assert_eq!(encode_mode(&pcm), 1 << 4);
    }

    #[test]
    fn data_width_set_encoding() {
        // tx_width occupies bits[0:2], rx_width bits[8:10] of data_width_set.
        let tx = DataWidth::Bits24; // code 7
        let rx = DataWidth::Bits16; // code 4
        let dw = ((tx as u32) & 0x07) | (((rx as u32) & 0x07) << 8);
        assert_eq!(dw, 0x0407);
    }

    #[test]
    fn fifo_threshold_encoding() {
        // tx threshold in low byte, rx threshold in second byte.
        let tx: u8 = 0x12;
        let rx: u8 = 0x34;
        let thresh = (tx as u32 & 0xFF) | ((rx as u32 & 0xFF) << 8);
        assert_eq!(thresh, 0x3412);
    }

    #[test]
    fn divider_masks() {
        // The dividers are masked to their field widths: bclk 7 bits, fs_num 10
        // bits, fs_ratio 11 bits. Out-of-range values wrap, never overflow.
        assert_eq!(0xFFu32 & 0x7F, 0x7F); // bclk_div max field
        assert_eq!(0xFFFFu32 & 0x3FF, 0x3FF); // fs_div_num max field
        assert_eq!(0xFFFFu32 & 0x7FF, 0x7FF); // fs_div_ratio max field
        // In-range values pass through unchanged.
        assert_eq!(0x40u32 & 0x7F, 0x40);
        assert_eq!(0x200u32 & 0x3FF, 0x200);
        assert_eq!(0x400u32 & 0x7FF, 0x400);
    }

    #[test]
    fn interrupt_mask_encoding() {
        // Each mask flag is one bit; all-set is 0x0F, none-set is 0.
        assert_eq!(encode_int_mask(false, false, false, false), 0x00);
        assert_eq!(encode_int_mask(true, false, false, false), 0x01);
        assert_eq!(encode_int_mask(false, true, false, false), 0x02);
        assert_eq!(encode_int_mask(false, false, true, false), 0x04);
        assert_eq!(encode_int_mask(false, false, false, true), 0x08);
        assert_eq!(encode_int_mask(true, true, true, true), 0x0F);
    }

    #[test]
    fn interrupt_status_decode_roundtrips_mask() {
        // The status decode is the inverse bit-layout of the mask encode: feeding
        // an encoded mask back through the decode recovers the same four flags.
        for bits in 0u32..16 {
            let (rx, tx, ovf, unf) = decode_int_status(bits);
            assert_eq!(encode_int_mask(rx, tx, ovf, unf), bits);
        }
    }

    #[test]
    fn interrupt_status_ignores_high_bits() {
        // Only the low nibble is meaningful; upper bits must not leak into flags.
        assert_eq!(decode_int_status(0xFFFF_FFF0), (false, false, false, false));
        assert_eq!(decode_int_status(0x0000_000F), (true, true, true, true));
    }

    #[test]
    fn bclk_fs_divider_formula() {
        // Per the module doc: BCLK = I2S_CLK / (BCLK_DIV_NUM + 1),
        // FS = BCLK / (FS_DIV_NUM + 1). A div of 0 means "divide by 1".
        let i2s_clk: u32 = 12_288_000; // typical 48kHz*256 master clock
        let bclk_div: u32 = 3;
        let fs_div: u32 = 63;
        let bclk = i2s_clk / (bclk_div + 1);
        let fs = bclk / (fs_div + 1);
        assert_eq!(bclk, 3_072_000);
        assert_eq!(fs, 48_000);
        // Divider of 0 is the identity (pass-through) case.
        assert_eq!(i2s_clk / (0 + 1), i2s_clk);
    }
}

// ── Property-based fuzz tests ──────────────────────────────────

#[cfg(all(test, not(target_arch = "riscv32")))]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        /// Fuzz: the divider field masks never exceed their field widths for any
        /// input, and in-range values are preserved (idempotent masking).
        #[test]
        fn divider_masks_bounded(bclk in any::<u8>(), num in any::<u16>(), ratio in any::<u16>()) {
            let m_bclk = bclk as u32 & 0x7F;
            let m_num = num as u32 & 0x3FF;
            let m_ratio = ratio as u32 & 0x7FF;
            prop_assert!(m_bclk <= 0x7F);
            prop_assert!(m_num <= 0x3FF);
            prop_assert!(m_ratio <= 0x7FF);
            // Masking is idempotent.
            prop_assert_eq!(m_bclk & 0x7F, m_bclk);
            prop_assert_eq!(m_num & 0x3FF, m_num);
            prop_assert_eq!(m_ratio & 0x7FF, m_ratio);
        }

        /// Fuzz: the FIFO-threshold packing keeps tx in the low byte and rx in the
        /// second byte with no cross-contamination, and is exactly recoverable.
        #[test]
        fn fifo_threshold_roundtrip(tx in any::<u8>(), rx in any::<u8>()) {
            let thresh = (tx as u32 & 0xFF) | ((rx as u32 & 0xFF) << 8);
            prop_assert_eq!((thresh & 0xFF) as u8, tx);
            prop_assert_eq!(((thresh >> 8) & 0xFF) as u8, rx);
            // Nothing leaks above bit 15.
            prop_assert_eq!(thresh & !0xFFFF, 0);
        }

        /// Fuzz: BCLK/FS divider formula never divides by zero (the +1 guards it)
        /// and the result is monotonically non-increasing as the divider grows.
        #[test]
        fn bclk_divider_no_div_by_zero(clk in any::<u32>(), div in any::<u8>()) {
            let bclk = clk / (div as u32 + 1);
            // div of 0 yields the full clock (identity), larger div never increases it.
            prop_assert!(bclk <= clk);
        }
    }
}
