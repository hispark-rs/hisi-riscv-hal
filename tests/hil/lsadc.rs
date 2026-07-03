use crate::{hal, pac};

/// LSADC scan-config register mapping (lsadc.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn lsadc_scan_config() {
    use hal::lsadc::{AdcChannel, AdcConfig, LsAdc};

    // SAFETY: sequential single-hart run; LSADC singleton not otherwise held.
    let mut adc = LsAdc::new(unsafe { hal::peripherals::Lsadc::steal() });
    let cfg = AdcConfig::default();
    adc.configure_scan(AdcChannel::Channel0, &cfg);

    // SAFETY: read-only MMIO load of the LSADC control register.
    let r = unsafe { &*pac::Lsadc::PTR };
    let ctrl0 = r.lsadc_ctrl_0().read();
    assert_ne!(ctrl0.channel().bits() & (1 << 0), 0, "LSADC channel-0 select bit not set");
    assert_eq!(ctrl0.equ_model_sel().bits(), cfg.averaging as u8, "LSADC averaging field mismatch");
    assert_eq!(ctrl0.sample_cnt().bits(), cfg.sample_count.bits(), "LSADC sample_cnt field mismatch");
}
