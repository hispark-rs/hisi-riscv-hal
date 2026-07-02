use crate::hal;

/// eFuse read path (efuse.rs / reset_demo).
#[cfg(feature = "chip-ws63")]
pub(crate) fn efuse_read_byte0_ok() {
    use hal::efuse::{EfuseByteAddress, EfuseDriver};

    // SAFETY: sequential single-hart run; EFUSE singleton not otherwise held.
    let mut efuse = EfuseDriver::new(unsafe { hal::peripherals::Efuse::steal() });
    let _ = efuse.read_byte(EfuseByteAddress::from_byte(0).unwrap());
}
