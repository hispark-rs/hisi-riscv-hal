use crate::hal;

/// I2S register liveness (i2s.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn i2s_version_live() {
    use hal::i2s::{I2sDriver, MasterConfig};

    // SAFETY: sequential single-hart run; I2S singleton not otherwise held.
    let i2s = I2sDriver::new_master(unsafe { hal::peripherals::I2s::steal() }, &MasterConfig::default());

    let ver = i2s.version();
    assert!(ver != 0 && ver != 0xFF, "I2S version register read an unsane value 0x{:02x} (block not clocked?)", ver);
}
