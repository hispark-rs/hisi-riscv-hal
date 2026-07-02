use crate::hal;

/// System reset-reason read-only decode (system.rs / examples/ws63/reset_demo).
#[cfg(feature = "chip-ws63")]
pub(crate) fn system_reset_reason_valid() {
    use hal::system::{ResetReason, System};

    // SAFETY: sequential single-hart run; these singletons not otherwise held.
    let system = unsafe {
        System::new(
            hal::peripherals::SysCtl0::steal(),
            hal::peripherals::GlbCtlM::steal(),
            hal::peripherals::CldoCrg::steal(),
        )
    };
    let reason = system.reset_reason();
    assert!(
        matches!(
            reason,
            ResetReason::PowerOn
                | ResetReason::ExternalPin
                | ResetReason::Watchdog
                | ResetReason::Software
                | ResetReason::BrownOut
                | ResetReason::Unknown
        ),
        "reset_reason() returned an out-of-range variant",
    );
}
