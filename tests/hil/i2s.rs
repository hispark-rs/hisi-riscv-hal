use crate::{hal, pac};

/// I2S register liveness (i2s.rs): version register must read a sane non-zero, non-FF value.
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2s_version_live() {
    use hal::i2s::{I2sDriver, MasterConfig};

    // SAFETY: sequential single-hart run; I2S singleton not otherwise held.
    let i2s = I2sDriver::new_master(unsafe { hal::peripherals::I2s::steal() }, &MasterConfig::default());

    let ver = i2s.version();
    assert!(ver != 0 && ver != 0xFF, "I2S version register read an unsane value 0x{:02x} (block not clocked?)", ver);
}

/// I2S master config register mapping: construct with a non-default MasterConfig and
/// verify each field lands in the correct register bit positions on real silicon.
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2s_master_config_registers() {
    use hal::i2s::{ChannelCount, DataWidth, I2sDriver, I2sMode, MasterConfig};

    let cfg = MasterConfig {
        mode: I2sMode::I2s,
        channels: ChannelCount::Four,
        data_width: DataWidth::Bits24,
        tx_fifo_threshold: 12,
        rx_fifo_threshold: 4,
        loopback: false,
    };

    // SAFETY: sequential single-hart run; I2S singleton not otherwise held.
    let _i2s = I2sDriver::new_master(unsafe { hal::peripherals::I2s::steal() }, &cfg);

    // SAFETY: read-only MMIO load of the I2S register block.
    let r = unsafe { &*pac::I2s::PTR };

    // data_width: Bits24 → raw field value 3.
    let dw = r.data_width_set().read().bits();
    assert_eq!(dw, 3u32, "I2S data_width field: expected 3 (Bits24), got {}", dw);

    // fifo_threshold: tx=12 (bits 7:0), rx=4 (bits 15:8) → 0x040C.
    let thresh = r.fifo_threshold().read().bits();
    assert_eq!(thresh, 0x040Cu32, "I2S fifo_threshold: expected 0x040C, got 0x{:04X}", thresh);

    // mode: master + I2S + 4ch → verify bit0 (master) is set.
    let mode = r.mode().read().bits();
    assert_ne!(mode & (1 << 0), 0, "I2S mode master bit not set");

    // i2s_crg: bclk_div_en + crg_clken both set.
    let crg = r.i2s_crg().read().bits();
    assert_eq!(crg & 0b11, 0b11, "I2S crg bclk-div-en + crg-clken not both set");

    // fs_div_num / bclk_div_num must be non-zero for a valid derive.
    let fs_div = r.i2s_fs_div_num().read().bits();
    let bclk_div = r.i2s_bclk_div_num().read().bits();
    assert_ne!(fs_div, 0, "I2S fs_div_num is zero — divider derivation failed?");
    assert_ne!(bclk_div, 0, "I2S bclk_div_num is zero — divider derivation failed?");

    // signed_ext = 1 (set by apply_common).
    assert_eq!(r.signed_ext().read().bits(), 1, "I2S signed_ext not set to 1");
}
