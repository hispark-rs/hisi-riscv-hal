use crate::hal;

/// On-die temperature sensor basic conversion path (tsensor.rs).
#[cfg(feature = "chip-ws63")]
pub(crate) fn tsensor_reads_in_range() {
    use hal::tsensor::TempSensor;

    // SAFETY: sequential single-hart run; TSENSOR singleton not otherwise held.
    let mut ts = TempSensor::new(unsafe { hal::peripherals::Tsensor::steal() });
    ts.enable();
    ts.start_conversion();
    let mut code = None;
    for _ in 0..1_000_000u32 {
        if let Some(c) = ts.read_raw() {
            code = Some(c);
            break;
        }
    }
    let code = code.expect("tsensor never asserted data-ready");
    assert!((114..=896).contains(&code), "tsensor code {code} outside the valid 114..=896 range");
}
