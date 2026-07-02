/// GADC register liveness (gadc.rs) — BS2X-only (`chip-bs21`).
#[cfg(feature = "chip-bs21")]
pub(crate) fn gadc_register_liveness() {
    use crate::pac;

    // SAFETY: read-only MMIO load of a GADC status register via the PAC.
    let r = unsafe { &*pac::Gadc::PTR };
    let status = r.rpt_gadc_data_3().read().bits();
    assert_ne!(status, 0xFFFF_FFFF, "GADC status read returned the all-ones bus-floating pattern");
}
