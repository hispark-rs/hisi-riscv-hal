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

/// I2S master config register mapping: raw write/readback on real silicon.
///
/// Uses I2sDriver::new_master(default) just to clock the block, then tests
/// register field positions via direct PAC writes, avoiding the full config
/// path (which requires specific clock tree state that may not be present
/// on the HIL board).
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2s_master_config_registers() {
    use hal::i2s::I2sDriver;

    // SAFETY: sequential single-hart run; I2S singleton not otherwise held.
    let _i2s = I2sDriver::new_master(unsafe { hal::peripherals::I2s::steal() }, &Default::default());

    // SAFETY: read-only MMIO load of the I2S register block.
    let r = unsafe { &*pac::I2s::PTR };

    // fifo_threshold: write tx=12 (bits 7:0), rx=4 (bits 15:8) → 0x040C
    unsafe {
        r.fifo_threshold().write(|w| w.bits(0x040C));
    }
    let thresh = r.fifo_threshold().read().bits();
    assert_eq!(thresh, 0x040C, "I2S fifo_threshold write/readback: expected 0x040C, got 0x{:04X}", thresh);

    // signed_ext
    unsafe {
        r.signed_ext().write(|w| w.bits(1));
    }
    assert_eq!(r.signed_ext().read().bits(), 1, "I2S signed_ext write/readback failed");

    // data_width_set: write 3 (=Bits24)
    unsafe {
        r.data_width_set().write(|w| w.bits(3));
    }
    assert_eq!(r.data_width_set().read().bits(), 3, "I2S data_width write/readback failed");

    // mode: set master bit (bit0)
    let orig_mode = r.mode().read().bits();
    unsafe {
        r.mode().write(|w| w.bits(orig_mode | 1));
    }
    assert_ne!(r.mode().read().bits() & 1, 0, "I2S mode master bit not set");

    // i2s_crg: bclk_div_en (bit0) + crg_clken (bit1)
    unsafe {
        r.i2s_crg().write(|w| w.bits(0b11));
    }
    assert_eq!(r.i2s_crg().read().bits() & 0b11, 0b11, "I2S crg clock-enable bits not both set");
}
